# Database Patterns

This codebase uses SurrealDB as its database. All database operations follow a consistent pattern through the `DbOperator` trait. See the [Testable Database Code with DbOperator](./TESTING.md#testable-database-code-with-dboperator) section for the current trait definition and how to write testable seam functions.

## Migrations

Each model has a `migrate` function that uses SurrealDB queries to define tables, fields, and indexes:

```rust
pub async fn migrate(db: &Surreal<Any>) -> Result<(), DbError> {
  db.query("
    USE NS remex DB remex;
    DEFINE TABLE IF NOT EXISTS client SCHEMAFULL;
    DEFINE FIELD IF NOT EXISTS client_name ON TABLE client TYPE string;
    ...
  ").await?.check()?;
  Ok(())
}
```

## Access Control

Endpoints and configurator users authenticate directly to SurrealDB with record-level permissions. The transitional server uses a BEARER grant scoped to a client record.

Endpoint access (`core/src/db/model/clients.rs`):

```sql
DEFINE ACCESS IF NOT EXISTS endpoint_access ON DATABASE TYPE RECORD
  SIGNUP { /* enrollment token verification + client creation */ }
  SIGNIN (SELECT * FROM client WHERE hardware_hash = $hardware_hash
          AND crypto::argon2::compare(secret, $secret)
          AND blocked != true)
  DURATION FOR TOKEN 1d;
```

Legacy server access (`server/SERVER_ARCHITECTURE.md`):

```sql
DEFINE ACCESS IF NOT EXISTS endpoint ON DATABASE TYPE BEARER FOR RECORD DURATION FOR GRANT 1d;
```

## Tables

- **client**: Client devices that connect to the system
- **execution**: Job execution records
- **group**: Client groupings
- **job**: Scheduled jobs to be executed
- **audit_log**: Audit trail for record changes on `job` and `group`
- **user**: Configurator users (for authentication)
- **refresh_token**: Configurator token refresh (singular table name)
- **config**: UI preferences and configuration (separate database)

## New Schema Design

### Job Table (v2)

The job table uses a computed field for execution status. The current migration is in `core/src/db/model/jobs.rs`. The logic looks at the **latest execution per endpoint** (not all executions):

```sql
DEFINE FIELD IF NOT EXISTS execution_status ON TABLE job TYPE object COMPUTED
  IF count((SELECT id FROM execution WHERE job_id = $this.id)) = 0
    THEN { Pending: {} }
  ELSE IF count((SELECT id FROM execution WHERE job_id = $this.id AND status = { Failed: {} } AND execution_start = (SELECT VALUE math::max(execution_start) FROM execution WHERE job_id = $this.id AND client_id = e.client_id))) > 0
    THEN { Failed: {} }
  ELSE IF count((SELECT id FROM execution WHERE job_id = $this.id AND status = { TimedOut: {} } AND execution_start = (SELECT VALUE math::max(execution_start) FROM execution WHERE job_id = $this.id AND client_id = e.client_id))) = count((SELECT client_id FROM execution WHERE job_id = $this.id GROUP BY client_id))
    THEN { TimedOut: {} }
  ELSE IF count((SELECT id FROM execution WHERE job_id = $this.id AND status = { Completed: {} } AND execution_start = (SELECT VALUE math::max(execution_start) FROM execution WHERE job_id = $this.id AND client_id = e.client_id))) = count((SELECT client_id FROM execution WHERE job_id = $this.id GROUP BY client_id))
    THEN { Completed: {} }
  ELSE { Running: {} }
  END;

DEFINE FIELD IF NOT EXISTS enabled ON TABLE job TYPE object FLEXIBLE DEFAULT { Draft: {} };
```

**Logic:**

1. No executions → `Pending`
2. Any endpoint's latest execution is `Failed` → `Failed`
3. ALL endpoints' latest executions are `TimedOut` → `TimedOut`
4. ALL endpoints' latest executions are `Completed` → `Completed`
5. Otherwise → `Running`

`enabled` is a stored, user-controlled object enum: `{ Draft: {} }`, `{ Enabled: {} }`, `{ Disabled: {} }`.

### Execution Table

```sql
DEFINE TABLE IF NOT EXISTS execution SCHEMAFULL;
DEFINE FIELD job_id ON TABLE execution TYPE record<job>;
DEFINE FIELD client_id ON TABLE execution TYPE record<client>;
DEFINE FIELD status ON TABLE execution TYPE object FLEXIBLE; -- ExecutionStatus enum
DEFINE INDEX idx_job_id ON TABLE execution COLUMNS job_id;
DEFINE INDEX idx_client_id ON TABLE execution COLUMNS client_id;
```

### Client Table

```sql
DEFINE TABLE IF NOT EXISTS client SCHEMAFULL;
DEFINE FIELD client_name ON TABLE client TYPE string;
DEFINE FIELD secret ON TABLE client TYPE string; -- hashed by endpoint_access SIGNUP, compared by SIGNIN
DEFINE FIELD hardware_hash ON TABLE client TYPE string;
DEFINE FIELD blocked ON TABLE client TYPE bool DEFAULT false;
DEFINE FIELD last_seen ON TABLE client TYPE option<datetime>;
DEFINE FIELD connection_history ON TABLE client TYPE array<object> DEFAULT [];
DEFINE EVENT IF NOT EXISTS trim_client_connection_history ON TABLE client
  WHEN $event = 'UPDATE'
  THEN {
    IF array::len($after.connection_history) > 100 THEN
      UPDATE $this.id SET connection_history = $after.connection_history[
        math::max(0, array::len($after.connection_history) - 100)..
      ];
    END;
  };
```

The `secret` field itself does **not** use `VALUE crypto::argon2::generate($value)`; hashing happens inside the `endpoint_access` SIGNUP/SIGNIN block.

### Audit Log Table

```sql
DEFINE TABLE IF NOT EXISTS audit_log SCHEMAFULL;
DEFINE FIELD table_name ON TABLE audit_log TYPE string;
DEFINE FIELD record_id ON TABLE audit_log TYPE record<job | client | group>;
DEFINE FIELD action ON TABLE audit_log TYPE string; -- CREATE, UPDATE, DELETE
DEFINE FIELD before_snapshot ON TABLE audit_log TYPE object FLEXIBLE;
DEFINE FIELD after_snapshot ON TABLE audit_log TYPE object FLEXIBLE;
DEFINE FIELD changed_at ON TABLE audit_log TYPE datetime DEFAULT time::now() READONLY;
DEFINE FIELD changed_by ON TABLE audit_log TYPE option<record<user | client>>;
```

Event triggers for audit logging currently exist on `job` and `group` only:

```sql
DEFINE EVENT IF NOT EXISTS audit_job ON TABLE job
WHEN $event IN ['CREATE', 'UPDATE', 'DELETE']
THEN {
  CREATE audit_log SET
    table_name = 'job',
    record_id = $after.id ?? $before.id,
    action = $event,
    before_snapshot = IF $event = 'CREATE' THEN {} ELSE $before END,
    after_snapshot = IF $event = 'DELETE' THEN {} ELSE $after END,
    changed_by = $auth.id;
};
```

### User Table (for Configurator)

```sql
DEFINE TABLE IF NOT EXISTS user SCHEMAFULL;
DEFINE FIELD username ON TABLE user TYPE string;
DEFINE FIELD email ON TABLE user TYPE string;
DEFINE FIELD password ON TABLE user TYPE string VALUE crypto::argon2::generate($value);
DEFINE FIELD created_at ON TABLE user TYPE datetime DEFAULT time::now() READONLY;
DEFINE FIELD updated_at ON TABLE user TYPE datetime VALUE time::now() READONLY;
DEFINE INDEX IF NOT EXISTS idx_email ON TABLE user COLUMNS email UNIQUE;
```

Access method for configurator:

```sql
DEFINE ACCESS IF NOT EXISTS configurator_access ON DATABASE TYPE RECORD
  SIGNUP (CREATE user SET username = $username, email = $email, password = $password)
  SIGNIN (
    IF $email != NONE AND $pass != NONE {
      SELECT * FROM user WHERE email = $email AND crypto::argon2::compare(password, $pass)
    } ELSE IF $refresh_token != NONE {
      SELECT * FROM user WHERE id = (
        SELECT VALUE user FROM refresh_token
        WHERE token = $refresh_token
          AND expires > time::now()
          AND (
            active = true
            OR (active = false AND revoked_at > time::now() - 1m)
          )
      )[0]
    } ELSE {
      THROW "Authentication failed: Invalid credentials or expired session token."
    }
  )
  DURATION FOR TOKEN 15m;
```

Access tokens for the configurator expire after **15 minutes** (`15m`). The configurator stores tokens in `sessionStorage` and mints server-side `refresh_token` records for session restore.

## Architecture Decisions

### 1. Computed Fields vs Stored Fields

- **execution_status** on job table is COMPUTED — derived from execution records at query time
- **enabled** on job table is stored — user-controlled state
- This separation allows the system to react to execution changes automatically

### 2. Offline Operation Strategy

- Endpoint maintains local cache of jobs and executions
- Jobs are cached locally for offline execution
- Executions created offline are stored locally, then synced to core on reconnect
- **No pull** of executions from core to endpoint (one-way sync)

### 3. Direct Database Access for Configurator and Endpoint

- The configurator connects directly to SurrealDB using record authentication (no REST API middleware)
- The endpoint connects directly to SurrealDB using `endpoint_access` record authentication
- The `remex_server` crate is transitional: it connects to SurrealDB, runs the core migrations, and still starts a legacy TCP listener, but it is no longer the primary path

### 4. Audit Trail Approach

- Using SurrealDB `DEFINE EVENT` for automatic audit logging
- Events fire on CREATE, UPDATE, DELETE operations
- Captures before/after snapshots using `$before` and `$after` variables
- `changed_by` captures `$auth.id` when available (user or endpoint token)

### 5. Connection Tracking

- Client connection history stored as embedded array (not separate table)
- Array limited to last 100 entries via `trim_client_connection_history` event
- Includes timestamp, event type, and IP address

## Typical Approaches

### When to Use COMPUTED Fields

Use for derived state that depends on related records:

```sql
DEFINE FIELD field_name ON TABLE table COMPUTED {
  -- Query related records and derive value
  RETURN computed_value;
};
```

### When to Use DEFINE EVENT

Use for side effects that must happen with the transaction:

- Audit logging (must not fail independently)
- Cascading updates
- Notifications

```sql
DEFINE EVENT event_name ON TABLE table
WHEN $event = "UPDATE"
THEN { /* side effects */ };
```

### When to Use Record Relationships

- Use `record<type>` fields for single references
- Use `array<record<type>>` for multiple references (like group members)
- Use relation tables only for many-to-many with additional metadata

### When to Use BEARER vs RECORD Access

- **BEARER FOR RECORD**: For the transitional server or services that need to act as a specific client record
- **TYPE RECORD**: For endpoints (`endpoint_access`) and configurator users (`configurator_access`) that sign in with credentials
- Both support token expiration and refresh
