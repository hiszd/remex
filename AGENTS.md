# AGENTS.md

## Directory Structure

The directory and project are laid out as such:
- `/core` - The shared library for all remex executables
- `/server` - The server executable (TCP communication only, no REST API)
- `/endpoint` - The endpoint executable and its related source code
- `/configurator` - The configurator Vue.js web application

## Systems Design

This system utilizes a client-server architecture at its core.

remex_core is a shared library that holds all shared logic for the server and the endpoint.

remex_server houses the application's central server which maintains connections via an encrypted TCP socket. It connects to the core SurrealDB database. The messages sent over the TCP socket from the server aren't for the purpose of sending database table queries to the endpoints, but instead is a centralized method of pointing the endpoints to the core database in the cloud. **Note: The REST API for configurator has been removed - configurator now connects directly to SurrealDB.**

remex_endpoint is the edge client that both connects with the remex_server and connects to the core SurrealDB database in the cloud. In addition to this, it also manages a local database for caching updates to be sent to the core database, and to reference for jobs that may need to be executed offline. It spawns several background tasks that monitor things like whether a job should be run yet, whether to respond to a server message, etc.

remex_configurator is a standalone Vue.js web application for the end user to create new configurations, modify existing ones, and check on the execution status of each job. It connects **directly to the core SurrealDB database** (same as endpoints) using SurrealDB's built-in authentication. User preferences and UI configuration are stored in a separate `config` database.

## CSS/SCSS Styles

All styles should use colors from the themes, and not hard-coded colors. If a theme color needs to be added, it should be added to the theme file.

## Database Patterns

This codebase uses SurrealDB as its database. All database operations follow a consistent pattern:

### DbOperator Trait

All database models implement the `DbOperator<T, U>` trait defined in `core/src/db/mod.rs`:

```rust
pub trait DbOperator<T, U>
where
  T: surrealdb::types::SurrealValue,
  U: surrealdb::types::SurrealValue,
{
  fn create(obj: U, db: &Surreal<Db>) -> impl Future<Output = Result<Option<T>, DbError>> + Send;
  fn read(id: String, db: &Surreal<Db>) -> impl Future<Output = Result<Option<T>, DbError>> + Send;
  fn push(&mut self, db: &Surreal<Db>) -> impl Future<Output = Result<(), DbError>> + Send;
  fn pull(&self, db: &Surreal<Db>) -> impl Future<Output = Result<Option<T>, DbError>> + Send;
  fn delete(&self, db: &Surreal<Db>) -> impl Future<Output = Result<(), DbError>> + Send;
}
```

- **create**: Insert a new record
- **read**: Fetch a record by ID
- **push**: Update (upsert) an existing record
- **pull**: Refresh a record from the database
- **delete**: Remove a record

### Migrations

Each model has a `migrate` function that uses SurrealDB queries to define tables, fields, and indexes:

```rust
pub async fn migrate(db: &Surreal<Any>) -> Result<(), DbError> {
  db.query("
    USE NS remex DB remex;
    DEFINE TABLE IF NOT EXISTS client SCHEMAFULL;
    DEFINE FIELD IF NOT EXISTS client_name ON TABLE client TYPE string;
    DEFINE FIELD IF NOT EXISTS secret ON TABLE client TYPE string VALUE crypto::argon2::generate($value);
    ...
  ").await?.check()?;
  Ok(())
}
```

### Access Control

Database access uses BEARER authentication with record-level permissions:

```sql
DEFINE ACCESS IF NOT EXISTS endpoint ON DATABASE TYPE BEARER FOR RECORD DURATION FOR GRANT 1d;
```

### Tables

- **client**: Client devices that connect to the system
- **execution**: Job execution records
- **group**: Client groupings
- **job**: Scheduled jobs to be executed
- **audit_log**: Audit trail for all record changes
- **user**: Configurator users (for authentication)
- **config**: UI preferences and configuration (separate database)

## New Schema Design

### Job Table (v2)

The job table now uses a computed field for execution status:

```sql
-- User-controlled field
DEFINE FIELD enabled ON TABLE job FLEXIBLE TYPE object DEFAULT { Draft: {} };
-- Values: { Draft: {} }, { Enabled: {} }, { Disabled: {} }

-- Computed field based on executions
DEFINE FIELD execution_status ON TABLE job FLEXIBLE TYPE object COMPUTED {
  LET $execs = (SELECT status FROM execution WHERE job_id = $this.id);
  IF array::len($execs) = 0 THEN RETURN { Pending: {} }; END IF;
  IF (SELECT VALUE status FROM $execs WHERE status = { Failed: {} }) THEN RETURN { Failed: {} }; END IF;
  IF array::len($execs) = (SELECT VALUE array::len((SELECT VALUE status FROM $execs WHERE status = { TimedOut: {} }))) THEN RETURN { TimedOut: {} }; END IF;
  IF array::len($execs) = (SELECT VALUE array::len((SELECT VALUE status FROM $execs WHERE status = { Completed: {} }))) THEN RETURN { Completed: {} }; END IF;
  RETURN { Running: {} };
};
```

**Logic:**
1. No executions → `Pending`
2. Any Failed → `Failed`
3. ALL TimedOut → `TimedOut`
4. ALL Completed → `Completed`
5. Any Running (no failures/timed out) → `Running`

### Execution Table

```sql
DEFINE TABLE IF NOT EXISTS execution SCHEMAFULL;
DEFINE FIELD job_id ON TABLE execution TYPE record<job>;
DEFINE FIELD client_id ON TABLE execution TYPE record<client>;
DEFINE FIELD status ON TABLE execution FLEXIBLE TYPE object; -- ExecutionStatus enum
DEFINE INDEX idx_job_id ON TABLE execution COLUMNS job_id;
DEFINE INDEX idx_client_id ON TABLE execution COLUMNS client_id;
```

### Client Table (Enhanced)

```sql
DEFINE TABLE IF NOT EXISTS client SCHEMAFULL;
DEFINE FIELD client_name ON TABLE client TYPE string;
DEFINE FIELD secret ON TABLE client TYPE string VALUE crypto::argon2::generate($value);
DEFINE FIELD hardware_hash ON TABLE client TYPE string;
DEFINE FIELD last_seen ON TABLE client TYPE datetime; -- NEW
DEFINE FIELD connection_history ON TABLE client TYPE array<object> DEFAULT []; -- NEW: {timestamp, event, ip_address}
DEFINE INDEX idx_hardware_hash ON TABLE client UNIQUE;
```

### Audit Log Table (NEW)

```sql
DEFINE TABLE IF NOT EXISTS audit_log SCHEMAFULL;
DEFINE FIELD table_name ON TABLE audit_log TYPE string;
DEFINE FIELD record_id ON TABLE audit_log TYPE record<job | client | group>;
DEFINE FIELD action ON TABLE audit_log TYPE string; -- CREATE, UPDATE, DELETE
DEFINE FIELD before_snapshot ON TABLE audit_log FLEXIBLE TYPE object;
DEFINE FIELD after_snapshot ON TABLE audit_log FLEXIBLE TYPE object;
DEFINE FIELD changed_at ON TABLE audit_log TYPE datetime DEFAULT time::now() READONLY;
DEFINE FIELD changed_by ON TABLE audit_log TYPE option<string>;
```

Event trigger for audit logging:
```sql
DEFINE EVENT audit_job ON TABLE job
WHEN $event IN ["CREATE", "UPDATE", "DELETE"]
THEN {
  CREATE audit_log SET
    table_name = "job",
    record_id = $after.id ?? $before.id,
    action = $event,
    before_snapshot = $before,
    after_snapshot = $after,
    changed_by = $auth.id;
};
```

### User Table (NEW - for Configurator)

```sql
DEFINE TABLE IF NOT EXISTS user SCHEMAFULL;
DEFINE FIELD username ON TABLE user TYPE string;
DEFINE FIELD email ON TABLE user TYPE string;
DEFINE FIELD password ON TABLE user TYPE string VALUE crypto::argon2::generate($value);
DEFINE FIELD created_at ON TABLE user TYPE datetime DEFAULT time::now() READONLY;
DEFINE FIELD updated_at ON TABLE user TYPE datetime VALUE time::now() READONLY;
DEFINE INDEX idx_email ON TABLE user COLUMNS email UNIQUE;
```

Access method for configurator:
```sql
DEFINE ACCESS configurator_access ON DATABASE TYPE RECORD
  SIGNUP (CREATE user SET username = $username, email = $email, password = crypto::argon2::generate($password))
  SIGNIN (SELECT * FROM user WHERE email = $email AND crypto::argon2::compare(password, $password))
  FOR TOKEN 1h;
```

## Architecture Decisions

### 1. Computed Fields vs Stored Fields
- **execution_status** on job table is COMPUTED - derived from execution records at query time
- **enabled** on job table is stored - user-controlled state
- This separation allows the system to react to execution changes automatically

### 2. Offline Operation Strategy
- Endpoint maintains local cache of jobs and executions
- Jobs are cached locally for offline execution
- Executions created offline are stored locally, then synced to core on reconnect
- **No pull** of executions from core to endpoint (one-way sync)

### 3. Direct Database Access for Configurator
- Configurator connects directly to SurrealDB (no REST API middleware)
- Uses SurrealDB's built-in authentication (DEFINE ACCESS ... TYPE RECORD)
- Server's web API has been removed to simplify architecture

### 4. Audit Trail Approach
- Using SurrealDB DEFINE EVENT for automatic audit logging
- Events fire on CREATE, UPDATE, DELETE operations
- Captures before/after snapshots using $before and $after variables
- Changed_by captures $auth.id when available (user or endpoint token)

### 5. Connection Tracking
- Client connection history stored as embedded array (not separate table)
- Array limited to last 100 entries (managed by event or application)
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
- **BEARER FOR RECORD**: For endpoints/services that need to act as a specific record
- **TYPE RECORD**: For configurator users that sign in with credentials
- Both support token expiration and refresh

## Communication Protocol

### Packet System

Messages are fragmented into 128-byte fixed-size packets for transmission over TCP:
- **Packet size**: 128 bytes total
- **Payload**: 126 bytes per packet (2 bytes for packet metadata)
- **Header**: `[packet_number, total_packets]`

### Message Contents Types

Messages are classified by their first character prefix:
- **`0` prefix**: Command - executable instructions
- **`1` prefix**: Secret - sensitive data (credentials, tokens)
- **Other**: Log - general logging information

### Stack Allocation

Uses `heapless::Vec` for fixed-capacity, no-heap allocations to ensure memory safety in constrained environments.

## Key Dependencies

| Dependency | Version | Purpose |
|------------|---------|---------|
| actix | 0.13 | Actor system for async message passing |
| surrealdb | 3 | Database with kv-surrealkv and protocol-ws |
| tokio | 1.38.1 | Async runtime |
| aes-gcm | 0.10.3 | Encryption for TCP socket communication |
| heapless | 0.9.2 | Fixed-capacity collections |
| chrono | 0.4.38 | Date/time handling |
| tracing | 0.1.40 | Structured logging |
| uuid | 1.6 | Unique identifiers (v4, v7) |

## Error Handling

This codebase uses two error handling approaches:

### thiserror

For custom error types with enum variants:

```rust
#[derive(thiserror::Error, Debug)]
pub enum DbError {
  #[error(transparent)]
  SurrealDb(#[from] surrealdb::Error),
  #[error("Operation failed: {0}")]
  OperationFailed(String),
}
```

### anyhow

For general application error handling with context propagation.

When adding new errors, prefer `thiserror` for domain-specific errors and `thiserror::Error` derive macro.

## Implementation Plan

### Phase 1: Cleanup (High Priority)

**Task 1.1: Remove deprecated enums from `core/src/db/model/jobs.rs`** ✅ COMPLETED
- Remove `JobSuccessStatus` enum (line 31-37)
- Remove `JobStatus` enum (line 50-66)
- Update any remaining references

**Task 1.2: Remove server web API** ✅ COMPLETED
- Remove `server/src/web/` directory entirely
- Keep `server/src/lib.rs` and `server/src/main.rs` for TCP communication
- Remove web-related dependencies from `server/Cargo.toml`

**Task 1.3: Update AGENTS.md** ✅ COMPLETED
- Document new architecture decisions
- Remove references to old JobStatus

### Phase 2: Database Schema Enhancements (High Priority) ✅ COMPLETED

**Task 2.1: Add indexes to execution table** (`core/src/db/model/executions.rs`) ✅ COMPLETED ✅ COMPLETED
```sql
DEFINE INDEX IF NOT EXISTS idx_job_id ON TABLE execution COLUMNS job_id;
DEFINE INDEX IF NOT EXISTS idx_client_id ON TABLE execution COLUMNS client_id;
```

**Task 2.2: Create audit_log table** (`core/src/db/model/audit.rs`) ✅ COMPLETED
- Fields: table_name, record_id, action, before_snapshot, after_snapshot, changed_at, changed_by

**Task 2.3: Add DEFINE EVENT triggers for audit logging** ✅ COMPLETED
On job, client, and group tables for CREATE/UPDATE/DELETE operations.

**Task 2.4: Add client connection tracking** (`core/src/db/model/clients.rs`) ✅ COMPLETED
- Add `last_seen: datetime` field
- Add `connection_history: array<object>` with structure: timestamp, event, ip_address
- Add logic to limit array to last 100 entries (TODO in EVENT or application code)
- Updated Client and ClientData structs

### Phase 3: Configurator Authentication (Medium Priority) ✅ COMPLETED

**Task 3.1: Create User table** (`core/src/db/model/users.rs`) ✅ COMPLETED
- Fields: username, email, password (argon2 hashed), created_at, updated_at
- Unique index on email

**Task 3.2: Setup DEFINE ACCESS for configurator** ✅ COMPLETED
- TYPE RECORD with SIGNUP/SIGNIN clauses
- FOR TOKEN with 1h expiration
- Defined in users.rs migration

**Task 3.3: Create config database tables** (`core/src/db/model/config.rs`) ✅ COMPLETED
- Add `global_config` table for global settings
- Add `user_config` table for per-user preferences

### Phase 4: Documentation (Medium Priority) ✅ COMPLETED

**Task 4.1: Verify AGENTS.md is complete** ✅ COMPLETED
- New Schema Design section ✓
- Architecture Decisions section ✓
- Typical Approaches section ✓
- Implementation Plan section ✓ (with completed tasks marked)

### Phase 5: Configurator Updates (Lower Priority - Future)

**Task 5.1: Update Vue.js views**
- Update `Job.vue` to use `execution_status` and `enabled` fields
- Update `JobsView.vue` and `JobDetailsView.vue` for new schema
- Add status badges for: Pending, Running, Completed, Failed, TimedOut

**Task 5.2: Implement login UI**
- Create login/registration views
- Use SurrealDB's record access for authentication
- Store bearer token in Vue app state

### Phase 6: Verification (Lower Priority)

**Task 6.1: Verify endpoint sync**
- Test offline execution storage
- Test sync to core on reconnect
- Verify audit logs are created correctly

**Task 6.2: Test audit trail**
- Create/update/delete records
- Verify audit_log entries are created
- Test querying audit_log for specific records
