# AGENTS.md

## Directory Structure

The directory and project are laid out as such:

- `/core` - The shared library for all remex executables
- `/server` - The server executable (TCP communication only, no REST API)
- `/endpoint` - The endpoint executable and its related source code
- `/macros` - proc-macro crate for derive macros
- `/configurator` - The configurator Vue.js web application
- `/docs` - Architecture documents and ADRs

## Systems Design

This system utilizes a client-server architecture at its core.

remex_core is a shared library that holds all shared logic for the server and the endpoint.

remex_server houses the application's central server which maintains connections via an encrypted TCP socket. It connects to the core SurrealDB database. The messages sent over the TCP socket from the server aren't for the purpose of sending database table queries to the endpoints, but instead is a centralized method of pointing the endpoints to the core database in the cloud. **Note: The REST API for configurator has been removed - configurator now connects directly to SurrealDB.**

remex_endpoint is the edge client that both connects with the remex_server and connects to the core SurrealDB database in the cloud. In addition to this, it also manages a local database for caching updates to be sent to the core database, and to reference for jobs that may need to be executed offline. It spawns several background tasks that monitor things like whether a job should be run yet, whether to respond to a server message, etc.

remex_configurator is a standalone Vue.js web application for the end user to create new configurations, modify existing ones, and check on the execution status of each job. It connects **directly to the core SurrealDB database** (same as endpoints) using SurrealDB's built-in authentication. User preferences and UI configuration are stored in a separate `config` database.

## Endpoint Architecture

### Background Tasks

The endpoint spawns **5 background tasks** in `endpoint/src/main.rs` (spawned during startup, then main sleeps forever):

| Task | File | Function | Purpose |
|---|---|---|---|
| Database connector | `endpoint/src/db_connector.rs` | `run()` | Connects to remote SurrealDB using bearer token, sends DB handle via `watch` channel |
| Server message loop | `endpoint/src/async_tasks/server_msg.rs` | `server_msg_loop()` | TCP connection to remex_server — ping/pong, sign-in/sign-up, receives bearer token and server URL |
| Job scheduler | `endpoint/src/async_tasks/jobs/scheduler.rs` | `run()` | BinaryHeap-based job queue — receives `JobQueueMessage`s, fires `Immediate` / `Scheduled` jobs |
| Remote monitor | `endpoint/src/async_tasks/jobs/monitor.rs` | `run()` | Connects to remote DB, sets up LIVE SELECT on `job` and `group` tables, reacts to changes by injecting jobs |
| Execution sync loop | `endpoint/src/async_tasks/jobs/sync.rs` | `execution_sync_loop()` | Every 30s: pushes unsynced local executions to remote DB; every 6h: cleans up old synced executions |

**Message flow between tasks:**

| Producer | Channel | Consumer | Message Type |
|---|---|---|---|
| `server_msg_loop` | `db_token_tx` (mpsc) | `db_connector` | `(BearerGrantResponse, String)` — bearer token + server URL |
| `db_connector` | `db_handle_tx` (watch) | `monitor`, `execution_sync_loop` | `Option<Surreal<Client>>` — remote DB handle (or `None`) |
| `server_msg_loop` | `monitor_cmd_tx` (mpsc) | `monitor` | `MonitorCommand::SetClientId(String)` |
| `monitor` | `job_injection_tx` (mpsc) | `scheduler` | `JobQueueMessage` — `Immediate`, `Scheduled`, `Remove`, `SyncFromRemote` |
| `scheduler` | spawns tasks | `execute_job()` | Job + client_id passed via async closure |

The `jobs` module (`endpoint/src/async_tasks/jobs/`) contains four sub-modules:

| Module | File | Key Exports |
|---|---|---|
| `scheduler` | `jobs/scheduler.rs` | `run(rx)` |
| `monitor` | `jobs/monitor.rs` | `run(cmd_rx, job_injection_tx, db_handle_rx)`, `MonitorCommand` |
| `sync` | `jobs/sync.rs` | `full_sync()`, `sync_groups()`, `sync_and_refill_queue()`, `sync_job_to_cache()`, `execution_sync_loop()` |
| `execution` | `jobs/execution.rs` | `execute_job()`, `should_skip_job()`, `mark_job_completed()`, `validate_shell()`, `run_command()` |

`JobQueueMessage` variants:

| Variant | Meaning |
|---|---|
| `Immediate { job, client_id }` | Execute the job right now |
| `Scheduled { job, execution_time, client_id }` | Execute the job at `execution_time` (an `Instant`) |
| `Remove { id }` | Remove a job from the scheduler queue by its `RecordId` |
| `SyncFromRemote` | Clear the entire scheduler queue |

### Local Database Structure

The endpoint runs an embedded SurrealDB (SurrealKV) with two logical databases inside the same engine:

| DB | Tables | Purpose |
|---|---|---|
| `remex` / `endpoint` | `session`, `last_action` | Utility tables (no remote counterpart) |
| `remex` / `remex` | `job` (cache), `execution` (cache) | Local caches of remote tables for offline operation |

**Init flow** (`endpoint/src/db.rs`):

1. `get_local_remex()` — lazily initializes `LOCAL_DB` (single `Surreal<Db>` instance backed by `surrealkv::Ds`), sets NS `remex` DB `remex`
2. `get_local_endpoint()` — same `LOCAL_DB`, sets NS `remex` DB `endpoint`
3. `migrate()` — runs migrations in order:
   - `Session::migrate()` (DB `endpoint`, table `session`)
   - `LastAction::migrate()` (DB `endpoint`, table `last_action`)
   - `JobCache::migrate()` (DB `remex`, table `job`)
   - `ExecutionCache::migrate()` (DB `remex`, table `execution`)

**Local tables:**

| Table | DB | Struct | Fields | Adapter |
|---|---|---|---|---|
| `session` | `endpoint` | `Session` | `client_id`, `client_name`, `hardware_hash`, `db_addr`, `tkn`, `secret`, `groups` | `SurrealSessionRepo` |
| `last_action` | `endpoint` | `LastAction` | `task_name`, `last_run` | (raw queries via `should_skip`/`record`/`cleanup_old`) |
| `job` (cache) | `remex` | `JobCache` | `job_id` (string), `job_info` (Job), `completed` (bool) | `SurrealJobCacheRepo` |
| `execution` (cache) | `remex` | `ExecutionCache` | `execution_id` (string), `execution_info` (Execution), `synced` (bool) | `SurrealExecutionCacheRepo` |

**Caching pattern:**

```
REMOTE (cloud)                    LOCAL (endpoint surrealkv)
─────────────────                 ────────────────────────────
job  ──sync_job_to_cache()──▶     job (JobCache)
                                    • job_id      — remote record id as string
                                    • job_info    — full serialised Job
                                    • completed   — local execution flag

execution ──execute_job()──▶     execution (ExecutionCache)
                                    • execution_id   — remote record id as string
                                    • execution_info — full serialised Execution
                                    • synced         — false until pushed to remote
```

Key properties:
- **JobCache** is **one-way pull**: fetched from remote, stored locally — never synced back
- **ExecutionCache** is **one-way push**: created offline, marked `synced: false`, pushed to remote by `execution_sync_loop`
- **Session** and **LastAction** are purely local with no remote counterpart

## CSS/SCSS Styles

All styles should use colors from the themes, and not hard-coded colors. If a theme color needs to be added, it should be added to the theme file.

### Global Design System (App.vue)

All common UI patterns are defined as global styles in `App.vue`. Use these classes in templates instead of redefining styles in component scoped styles.

#### CSS Variables
- `--color-border`: Border color (maps to `--background-400`)

#### Buttons
- `.btn-primary`: Accent-colored action button
- `.btn-secondary`: Outlined accent button
- `.btn-danger`: Red danger/delete button
- `.btn-ghost`: Subtle bordered button

All buttons share: `padding: 0.625rem 1.25rem`, `border-radius: 0.5rem`, `font-size: 0.875rem`, `font-weight: 600/700`, hover/disabled states.

#### Cards
- `.card`: Standard card with `background: var(--background-300)`, `border-radius: 1rem`, `padding: 1.5rem`, subtle shadow

#### Badges
- `.status-badge`: For execution status (pending, running, completed, failed, timedout)
- `.state-badge`: For enabled state (draft, enabled, disabled)
Both share: `padding: 0.125rem 0.625rem`, `border-radius: 9999px`, `font-size: 0.75rem`, `font-weight: 600`

#### Forms
- `.form-input`: Standard input/textarea/select styling with focus ring
- `.form-label`: Uppercase label style (`font-size: 0.8rem`, `font-weight: 700`, `letter-spacing: 0.025em`)
- `.info-label`: Smaller label for read-only detail views (`font-size: 0.7rem`, `letter-spacing: 0.05em`)

#### Layout
- `.page`: Page container with `padding: 2rem 1.5rem`, `gap: 2rem`
- `.page-header`: Header container
- `.page-title`: List page titles (`font-size: 1.75rem`, `font-weight: 800`)
- `.page-subtitle`: Subtitle text (`font-size: 0.9rem`, `color: var(--text-500)`)
- `.back-link`: Navigation back link
- `.header-main`: Detail page header with title + actions
- `.title-group`: Title + record ID grouping
- `.section-header`: Section header with h2 + optional action button
- `.info-grid`: Responsive grid for detail view info items
- `.info-item`: Single info item with label + value

#### Utilities
- `.empty-state`: Empty list/content placeholder
- `.state-card`: Loading/error/empty state container
- `.monospace`: Monospace text styling
- `.spinner` + `@keyframes spin`: Loading spinner
- `.details-btn`: Icon-only "view details" button
- `.assignment-list`, `.assignment-item`, `.assignment-badges`, `.assignment-name`: Assignment list patterns

#### Modals
- `.modal-overlay`: Fixed overlay with `backdrop-filter: blur(4px)`, `z-index: 9999`
- `.modal-content`: Centered modal card
- `.modal-actions`: Action button row

## Icon Library

**Location**: `/configurator/src/components/icons/`

**Naming Convention**: `Icon[Name].vue` (PascalCase)

**Guidelines for Creating Icons**:
- Use `viewBox="0 0 [width] [height]"` for scalable icons
- Default sizes: 16px for button icons, 20px for standard icons
- Use `stroke="currentColor"` and `fill="none"` for line-based icons
- Use `stroke-width="1.5"` for consistent line weight
- Use `stroke-linecap="round"` and `stroke-linejoin="round"` for smooth corners
- Always use `currentColor` to inherit text color and support light/dark themes

**Using Icons in Components**:
- Icon containers must have a theme-aware color set (e.g., `color: var(--text)`)
- This ensures icons automatically adapt to light/dark mode
- Example:
  ```scss
  .icon-container {
    color: var(--text);
    // Icons will inherit this color via currentColor
  }
  ```

**Available Icons**:

### IconViewDetails.vue (16x16)
Document icon for "view details" actions.

```vue
<template>
  <svg width="16" height="16" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
    <rect x="3" y="2" width="10" height="12" rx="1"/>
    <path d="M6 6h4M6 9h4"/>
  </svg>
</template>
```

### IconGroup.vue (20x20)
Overlapping people icon for groups.

```vue
<template>
  <svg width="20" height="20" viewBox="0 0 20 20" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
    <circle cx="7" cy="7" r="3"/>
    <path d="M2 17v-1a4 4 0 0 1 4-4h2a4 4 0 0 1 4 4v1"/>
    <circle cx="14" cy="8" r="2"/>
    <path d="M18 17v-1a3 3 0 0 0-3-3h-1"/>
  </svg>
</template>
```

### IconClient.vue (20x20)
Monitor icon for clients.

```vue
<template>
  <svg width="20" height="20" viewBox="0 0 20 20" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
    <rect x="3" y="4" width="14" height="10" rx="1"/>
    <path d="M8 17h4M10 14v3"/>
  </svg>
</template>
```

### IconJob.vue (16x16)
Briefcase icon for jobs.

```vue
<template>
  <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
    <rect x="2" y="7" width="20" height="14" rx="2" ry="2" />
    <path d="M16 21V5a2 2 0 00-2-2h-4a2 2 0 00-2 2v16" />
  </svg>
</template>
```

**Usage Example**:

```vue
<script setup>
import IconViewDetails from '@/components/icons/IconViewDetails.vue'
</script>

<template>
  <button>
    <IconViewDetails />
    View Details
  </button>
</template>
```

## Database Patterns

This codebase uses SurrealDB as its database. All database operations follow a consistent pattern through the `DbOperator` trait. See the [Testable Database Code with DbOperator](#testable-database-code-with-dboperator) section for the current trait definition and how to write testable seam functions.

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
- **refresh_tokens**: Configurator token refresh
- **config**: UI preferences and configuration (separate database)

## New Schema Design

### Job Table (v2)

The job table now uses a computed field for execution status:

```sql
-- User-controlled field
DEFINE FIELD enabled ON TABLE job TYPE object FLEXIBLE DEFAULT { Draft: {} };
-- Values: { Draft: {} }, { Enabled: {} }, { Disabled: {} }

-- Computed field based on executions
DEFINE FIELD execution_status ON TABLE job TYPE object FLEXIBLE COMPUTED {
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
DEFINE FIELD status ON TABLE execution TYPE object FLEXIBLE; -- ExecutionStatus enum
DEFINE INDEX idx_job_id ON TABLE execution COLUMNS job_id;
DEFINE INDEX idx_client_id ON TABLE execution COLUMNS client_id;
```

### Client Table (Enhanced)

```sql
DEFINE TABLE IF NOT EXISTS client SCHEMAFULL;
DEFINE FIELD client_name ON TABLE client TYPE string;
DEFINE FIELD secret ON TABLE client TYPE string VALUE crypto::argon2::generate($value);
DEFINE FIELD hardware_hash ON TABLE client TYPE string;
DEFINE FIELD last_seen ON TABLE client TYPE datetime;
DEFINE FIELD connection_history ON TABLE client TYPE array<object> DEFAULT [];
```

### Audit Log Table

```sql
DEFINE TABLE IF NOT EXISTS audit_log SCHEMAFULL;
DEFINE FIELD table_name ON TABLE audit_log TYPE string;
DEFINE FIELD record_id ON TABLE audit_log TYPE record<job | client | group>;
DEFINE FIELD action ON TABLE audit_log TYPE string; -- CREATE, UPDATE, DELETE
DEFINE FIELD before_snapshot ON TABLE audit_log TYPE object FLEXIBLE;
DEFINE FIELD after_snapshot ON TABLE audit_log TYPE object FLEXIBLE;
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

### User Table (for Configurator)

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

## SurrealDB Pitfalls

### `to_sql()` Adds the Table Prefix

`RecordId::to_sql()` returns `"table_name:key"`, not just `"key"`. This is easy to forget when constructing strings for field comparison:

```rust
let id = RecordId::new("job", "abc123");
assert_eq!(id.to_sql(), "job:abc123"); // NOT "abc123"
```

Test code often mirrors this with a helper:
```rust
fn job_sql_id(key: &str) -> String {
  format!("job:{key}")
}
```

### `UPDATE ... CONTENT` vs `UPDATE ... MERGE`

The `impl_surreal_db_operator!` macro uses different SurrealQL keywords for `create` and `update`:

- **`CREATE ... CONTENT $data`** — replaces the entire document body. Any field not present in `$data` is removed. This can break SCHEMAFULL tables if required fields are omitted.
- **`UPDATE $id MERGE $data`** — partial update. Only the fields present in `$data` are changed. The record `id` is always included in the response.

In SurrealDB v3, `UPDATE ... CONTENT` may return the document body **without** the record `id`, causing deserialization failures on structs that require an `id: RecordId` field. This is why `update` uses `MERGE`, not `CONTENT`.

### Schemafull Tables Require All Fields

SCHEMAFULL tables (all core and endpoint tables except `config`) require that any field defined via `DEFINE FIELD` must be present on `CREATE` unless it has a `DEFAULT`. For example, the `execution` table has zero optional field definitions — every field (`output`, `command`, `exit_code`, `execution_start`, `execution_end`, etc.) must be provided.

### Local and Remote Tables Share Names

The endpoint's local cache `job` table and the remote `job` table are both in `remex.remex` on their respective SurrealDB instances. They have different schemas:

| Table | Location | Schema |
|---|---|---|
| `job` (remote) | Cloud SurrealDB | `job_name`, `job_shell`, `job_command`, `job_type`, `execution_status`, `enabled`, ... |
| `job` (cache) | Local SurrealKV | `job_id` (string), `job_info` (FLEXIBLE), `completed` (bool) |

The same applies to `execution`. The endpoint's migration only defines the cache fields, so queries must match the table's actual schema on the instance being queried.

### In-Memory Adapter Has UPSERT Semantics

The in-memory adapter's `update` calls `HashMap::insert`, which **creates** a record if it doesn't exist. The real `UPDATE ... MERGE` in SurrealDB may error or return empty for non-existent records. Tests using the in-memory adapter can mask production errors where updates target missing records.

### (Configurator) `enabled` in `@tanstack/vue-query` is Evaluated Once

Passing a plain boolean like `enabled: !!job.value` evaluates at setup time and is never re-evaluated. Use a function `enabled: () => !!job.value` for reactive behavior, or restructure the query to not depend on another query's result.

**Wrong** — `enabled` is locked at `false` forever:
```ts
const { data: job } = useQuery({ queryKey: ["job", id], queryFn: () => getJob(client, id) })
const { data: execs } = useQuery({
  queryKey: ["executions", id],
  queryFn: () => getExecutionsForJob(client, job.value!.id),
  enabled: !!job.value,  // BUG: never re-evaluated
})
```

**Right** — make the executions query independent:
```ts
const { data: job } = useQuery({ queryKey: ["job", id], queryFn: () => getJob(client, id) })
const { data: execs } = useQuery({
  queryKey: ["executions", id],
  queryFn: () => getExecutionsForJob(client, rid(id)),  // uses route param directly
})
```

### (Configurator) SurrealDB JS SDK `RecordId.toString()` Double-Wraps Angle Brackets

The SDK v2.0.3's `escapeIdent` function wraps record ID values in `⟨...⟩` when they contain non-ASCII characters. If the `id` part already contains `⟨...⟩` (as with UUID-based record IDs from SurrealDB), `toString()` double-wraps them: `execution:⟨⟨uuid\⟩⟩`.

When round-tripping a RecordId through URL serialization (`router.push(\`/path/${recordId}\`)`) and back through `rid()`, the `rid()` helper must strip the outer brackets before constructing a new `RecordId`:

```ts
export function rid(id: string): RecordId {
  const sep = id.indexOf(":")
  let idPart = id.slice(sep + 1)
  if (idPart.startsWith("\u27e8") && idPart.endsWith("\u27e9")) {
    idPart = idPart.slice(1, -1)
  }
  return new RecordId(id.slice(0, sep), idPart)
}
```

Always use `rid()` (not `new RecordId(...)`) when parsing record IDs from route params or user input.

## Lessons Learned

### 1. `enabled` in Query Libraries Must Be Reactive

TanStack Vue Query v5 evaluates `enabled` once inside a `computed()` at setup time. Passing a plain boolean like `!!job.value` evaluates immediately and is never re-evaluated, even if `job.value` changes later. This creates a subtle deadlock — a query dependent on another query's data may never fire on fresh page loads when that data hasn't been cached yet.

**Always make queries independent of each other when possible.** A query's `queryFn` should derive its parameters from stable sources (route params, refs initialized before queries) rather than another query's result. If you must chain queries, pass `enabled` as a function: `enabled: () => !!job.value`.

### 2. Don't Trust SDK `toString()` for Round-Tripping

The SurrealDB JS SDK v2.0.3's `RecordId.toString()` produces a SurrealQL-compatible string that wraps UUID IDs in `⟨...⟩` via `escapeIdent`. If you parse that string back into a `RecordId` via `new RecordId(table, idPart)`, the new `RecordId`'s `toString()` will wrap the already-bracketed `id` again, producing `execution:⟨⟨uuid\⟩⟩` — a value that doesn't match any record in the database.

**The `rid()` helper exists precisely to handle this.** Always route record ID string parsing through `rid()` rather than directly calling `new RecordId()`. See the pitfall above for the implementation.

### 3. Debug with Logging, Fix by Simplifying

Both bugs were diagnosed by adding `console.log` to trace the actual values flowing through the system:
- In the execution detail view, logging `execId`, `String(recordId)`, and `recordId.id` revealed that the queried ID didn't match database records
- In the job details view, checking whether the executions query was ever firing (it wasn't) revealed the `enabled` deadlock

Once diagnosed, both fixes simplified the code — removing the `enabled` option entirely in one case, and making `rid()` more robust in the other. If a fix complicates the code, it's probably the wrong fix.

### 4. URL Round-Trips Can Corrupt Typed Values

Passing SurrealDB `RecordId` objects through URL serialization (`router.push(\`/path/${recordId}\`)`) and back (`route.params.id` → `rid()`) is inherently lossy. The SDK's string representation includes SurrealQL escaping that isn't idempotent. Prefer to pass the id as its raw parts (`table:id` without escaping) or keep a reference to the original `RecordId` object in a store/cache rather than re-parsing from the URL.

## Key Dependencies

| Dependency | Version | Purpose |
|------------|---------|---------|
| actix | 0.13 | Actor system for async message passing |
| surrealdb | 3 | Database with kv-surrealkv and protocol-ws |
| tokio | 1.38.1 | Async runtime |
| aes-gcm | 0.10.3 | Encryption for TCP socket communication |
| chrono | 0.4.38 | Date/time handling |
| tracing | 0.1.40 | Structured logging |
| uuid | 1.6 | Unique identifiers (v4, v7) |

## Error Handling

### Hard Rule — Never Silently Discard Errors

There is absolutely no circumstance where errors should be silently discarded. Every error must either:
- **Log to the command line and continue execution** (with `tracing::error!`/`tracing::warn!`), or
- **Stop the program execution** if there is no appropriate fallback

**There are no exceptions.**

This means:
- ❌ **Never** use `let _ =` to swallow a `Result` or error-returning call
- ❌ **Never** use `.ok()` or `.unwrap_or_default()` to ignore errors without logging
- ✅ Use `let _ =` only for values that are genuinely not errors (e.g., `Sink` items, drop handles, or test cleanup where failure is acceptable)
- ✅ Always log the error context before continuing after a non-fatal failure

### Approaches

This codebase uses two error handling approaches:

#### thiserror

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

#### anyhow

For general application error handling with context propagation.

When adding new errors, prefer `thiserror` for domain-specific errors and `thiserror::Error` derive macro.

## Testing

### Test Framework

All tests use `#[tokio::test]` (async runtime). Assertions use standard `assert!`/`assert_eq!`/`assert_ne!`. There are no third-party assertion crates and no `[dev-dependencies]` — all test dependencies are already regular workspace deps.

### Test Module Placement

Tests are defined **inline** at the bottom of the source file they test, gated with `#[cfg(test)]`:

```rust
#[cfg(test)]
mod my_tests {
  // imports, helpers, #[tokio::test] functions
}
```

Module naming convention: `<name>_tests` (e.g., `sync_tests`, `execution_tests`). Production functions under test are accessed via `super::`.

### Running Tests

```bash
cargo test                    # all workspace tests
cargo test -p remex-core      # core tests only
cargo test -p remex-endpoint  # endpoint tests only
cargo test sync_tests         # tests matching module name
cargo test execution_tests    # execution seam tests
cargo test server_msg::tests  # session seam tests
```

### Testing Pattern: Seam Functions + In-Memory Adapter

Database-dependent logic is tested via the **seam function** pattern (see [Testable Database Code with DbOperator](#testable-database-code-with-dboperator)). Each seam function accepts `&dyn DbOperator<Record = X, Input = Y>` as a parameter, and test modules generate an in-memory adapter via `impl_in_memory_db_operator!`.

**Recipe:**

1. Extract the logic into a function accepting `&dyn DbOperator`
2. In the test module, do `impl_in_memory_db_operator!(InMemoryXRepo, RecordType, InputType, "table")`
3. Instantiate `InMemoryXRepo::new()` in the test
4. Call the seam function and assert on the result

**18 seam tests** exist across the endpoint:
- `sync_job_to_cache`: 5 tests (new job, unchanged preserves, changed resets, missing cache, multiple independence)
- `should_skip_job`: 4 tests (no cache, completed=false, completed=true, other cache)
- `mark_job_completed`: 4 tests (updates existing, no-op on missing, idempotent, doesn't affect others)
- `create_new_session_with_repo`: 2 tests (defaults, unique IDs)
- `persist_session_with_repo`: 3 tests (state, without client_id, full CRUD roundtrip)

## Testable Database Code with DbOperator

All database operations go through the `DbOperator` trait (`core/src/db/mod.rs`):

```rust
#[async_trait]
pub trait DbOperator: Send + Sync {
  type Record: Send + Sync + 'static;
  type Input: Send + Sync + 'static;

  async fn create(&self, input: Self::Input) -> Result<Self::Record, DbError>;
  async fn read(&self, id: &str) -> Result<Option<Self::Record>, DbError>;
  async fn update(&self, id: &str, input: Self::Input) -> Result<Self::Record, DbError>;
  async fn list(&self) -> Result<Vec<Self::Record>, DbError>;
  async fn delete(&self, id: &str) -> Result<(), DbError>;
}
```

### Seam Pattern

To make a function testable, extract it to accept `&dyn DbOperator` instead of directly querying SurrealDB:

```rust
// ❌ Not testable — tightly coupled to SurrealDB
async fn do_stuff(db: &Surreal<Db>, id: &str) -> Result<(), Error> {
  db.query("SELECT * FROM table WHERE field = $id").bind(("id", id)).await?;
  // ...
}

// ✅ Testable — accepts any adapter via &dyn DbOperator
async fn do_stuff(
  id: &str,
  repo: &dyn DbOperator<Record = MyRecord, Input = MyInput>,
) -> Result<(), Error> {
  let records = repo.list().await?;
  if let Some(record) = records.iter().find(|r| r.field == id) {
    repo.update(&record.id_as_key(), MyInput { ... }).await?;
  }
  Ok(())
}

// Production caller wires in the real adapter:
let repo = SurrealMyRepo { db: get_local_remex().await? };
do_stuff(&id, &repo).await?;
```

### In-Memory Adapter for Tests

Declarative macros in `core/src/db/adapters.rs` generate adapters:

```rust
use remex_core::impl_in_memory_db_operator;
impl_in_memory_db_operator!(InMemoryMyRepo, MyRecord, MyInput, "table_name");
```

**Requirements:**
- `MyRecord: Send + Sync + Clone + 'static`
- `MyInput: Send + Sync + Clone + 'static`
- `impl From<(String, MyInput)> for MyRecord` (for ID generation)

### Recipe: Writing a Testable Seam

1. **Define the seam function** — takes `&dyn DbOperator<Record = X, Input = Y>` as its last parameter
2. **Update the caller** — create the real adapter (`SurrealXRepo { db }`) and pass a reference
3. **Generate an in-memory adapter** — `impl_in_memory_db_operator!` in the test module
4. **Write tests** — instantiate `InMemoryXRepo::new()`, call the seam function, assert on results

### Reference Examples in this Repo

| Seam Function | File | Tests |
|---|---|---|
| `sync_job_to_cache` | `endpoint/src/async_tasks/jobs/sync.rs:135` | 5 tests at line 157 |
| `should_skip_job` | `endpoint/src/async_tasks/jobs/execution.rs:57` | 4 tests at line 223 |
| `mark_job_completed` | `endpoint/src/async_tasks/jobs/execution.rs:68` | 4 tests at line 223 |
| `create_new_session_with_repo` | `endpoint/src/async_tasks/server_msg.rs:213` | 2 tests at line 263 |
| `persist_session_with_repo` | `endpoint/src/async_tasks/server_msg.rs:237` | 3 tests at line 263 |

### Adapter Generation Macros

Both macros handle `id` generation differently:

- **`impl_surreal_db_operator!(pub Name, Record, Input, "table", "ns", "db")`**: Gets the real DB handle. `create` uses `CREATE table CONTENT $data` (auto-generates ID). `update` uses `UPDATE $id MERGE $data` for partial updates. `list` uses `SELECT * FROM table`.

- **`impl_in_memory_db_operator!(pub Name, Record, Input, "table")`**: `create` generates a UUID v4 string key, calls `From<(String, Input)>`. `update` replaces via HashMap insert (UPSERT semantics). `list` clones all HashMap values.

### Notes

- Use `list()` + iterator filtering for field-based lookups (the trait only supports `read` by record ID)
- `update` has UPSERT semantics in both adapters
- The in-memory adapter is **not persistent** — each test gets a fresh state
- Always clean up unused imports when refactoring (the caller may no longer need `Surreal`, `RecordId`, etc. directly)

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

### Phase 5: Configurator Updates (Lower Priority) ✅ COMPLETED

**Task 5.1: Update Vue.js views** ✅ COMPLETED

- Updated `Job.vue` to use `execution_status` (computed) and `enabled` fields
- Updated `JobsView.vue`, `JobDetailsView.vue`, `CreateJobView.vue` for new schema
- Updated `Group.vue`, `GroupDetailsView.vue`, `CreateGroupView.vue`, `GroupsView.vue` for new schema
- Updated `ClientDetailsView.vue`, `ClientsView.vue` for new schema
- Updated `DashboardView.vue` for new schema
- Added status badges for: Pending, Running, Completed, Failed, TimedOut
- All views now use actual DB field names (snake_case)
- Created `src/lib/model.ts` with full TypeScript types matching backend schema
- Created `src/lib/api.ts` with CRUD functions (getJobs, getJobById, createJob, updateJob, deleteJob, etc.)
- Added `extractEnumVariant()` and `formatEnumVariant()` helpers for SurrealDB object-typed enums
- Added `FIELD_LABELS` map and `fieldLabel()` helper for human-readable display names

**Task 5.2: Implement login UI** ✅ COMPLETED

- Created `LoginView.vue` with email/password form
- Created `RegisterView.vue` with username/email/password form
- Created `src/lib/auth.ts` with `useAuth()` composable (login, signup, logout, session restore)
- Uses SurrealDB's `signin()`/`signup()` with `configurator_access` record access
- Auth token stored in `localStorage` for persistence
- Session restore on app startup via `tryRestoreSession()`
- Router navigation guard redirects unauthenticated users to `/login`
- Sidebar shows username and logout button when authenticated
- Removed hardcoded root credentials from App.vue
- Full-screen auth layout (no sidebar) on login/register pages

### Phase 6: Verification (Lower Priority)

**Task 6.1: Verify endpoint sync**

- Test offline execution storage
- Test sync to core on reconnect
- Verify audit logs are created correctly

**Task 6.2: Test audit trail**

- Create/update/delete records
- Verify audit_log entries are created
- Test querying audit_log for specific records

## Agent skills

### Issue tracker

Issues are tracked in GitHub using the `gh` CLI. External PRs are not treated as a triage surface. See `docs/agents/issue-tracker.md`.

### Triage labels

Using default canonical triage labels. See `docs/agents/triage-labels.md`.

### Domain docs

Multi-context monorepo layout. See `docs/agents/domain.md`.

## Configurator Design System

The configurator uses a blue corporate design system with a collapsible sidebar + top bar layout.

### Layout Architecture

| Component | Role |
|---|---|
| `App.vue` | Root — provides DesignSidebar + DesignTopBar for all authenticated routes; auth pages (login/register) use full-screen layout |
| `DesignSidebar.vue` | Collapsible sidebar with icon + label nav items. Auto-collapses ≤768px. Dark bg: `#334155`. Blue accent active state with left border indicator |
| `DesignTopBar.vue` | Shows page title (from `route.meta.title`), hamburger toggle, optional search input |

### Route Meta

Each route defines `meta.title` used by DesignTopBar:

| Route | `meta.title` |
|---|---|
| `/` | "Dashboard" |
| `/jobs` | "Jobs" |
| `/jobs/new` | "Create Job" |
| `/jobs/:id` | "Job Details" |
| `/groups` | "Groups" |
| `/groups/new` | "Create Group" |
| `/groups/:id` | "Group Details" |
| `/clients` | "Clients" |
| `/clients/:id` | "Client Details" |

### Global CSS Classes (defined in App.vue)

**Layout:** `.page`, `.page-title`, `.page-subtitle`, `.header-main`, `.title-group`, `.header-actions`, `.section-header`, `.section-header-row`, `.section-actions`, `.back-link`, `.design-layout`, `.design-content`, `.design-main`

**Data display:** `.data-table`, `.data-table-card`, `.stats-grid`, `.stat-card`, `.stat-card-header`, `.stat-card-label`, `.stat-card-value`, `.stat-card-change`, `.stat-icon`

**Badges:** `.status-badge` (pending/running/completed/failed/timedout), `.state-badge` (draft/enabled/disabled)

**Forms:** `.form-input`, `.form-label`, `.info-label`, `.info-grid`, `.info-item`

**Buttons:** `.btn-primary` (blue fill), `.btn-secondary` (blue-tinted fill), `.btn-danger` (vibrant red fill), `.btn-ghost` (slate fill)

**Utilities:** `.card`, `.back-link`, `.empty-state`, `.empty-state-block`, `.empty-state-icon`, `.state-card`, `.monospace`, `.spinner`, `.modal-overlay`, `.modal-content`, `.modal-actions`, `.assignment-list`, `.assignment-item`, `.assignment-badges`, `.assignment-name`, `.details-btn`, `.search-input`, `.view-toggle`, `.hamburger-btn`, `.sidebar-overlay`, `.top-bar`, `.top-bar-left`, `.top-bar-right`, `.top-bar-title`, `.top-bar-breadcrumbs`

### View Patterns

| Pattern | Files |
|---|---|
| **List with view toggle** (table/grid) | `JobsView.vue`, `GroupsView.vue`, `ClientsView.vue` |
| **Detail with info-grid + sections** | `JobDetailsView.vue`, `GroupDetailsView.vue`, `ClientDetailsView.vue` |
| **Create form** (back-link + card form) | `CreateJobView.vue`, `CreateGroupView.vue` |
| **Dashboard** (stat cards + recent data-table) | `DashboardView.vue` |

### View Toggle Pattern

List views implement a table/grid toggle:
```vue
const viewMode = ref<"table" | "grid">("table")
```

When `viewMode === "table"`, render `data-table-card > data-table`. When `viewMode === "grid"`, render a `stats-grid` of clickable stat-card elements.

### Data Gaps (not currently in schema)

| Data Point | Where Needed | Schema Gap |
|---|---|---|
| Online/offline status per client | Dashboard stat card, client list | `last_seen` exists but no live status |
| Job execution count | Dashboard, client details, group details | Requires aggregation query on `execution` table |
| Job last execution time | Dashboard, job details | Not stored on job — derived from executions |
| Execution duration | Job details | `execution_start`/`execution_end` exist but not surfaced |
| Job description | Job detail | No field exists |
| Job tags/labels | Job list filter | No field exists |
| Next scheduled run | Job detail | Cron/schedule not yet in schema |
| Client IP / location | Client details | `connection_history` has ip but not surfaced |

### Removed Files (post-redesign)

| File | Reason |
|---|---|
| `AppSidebar.vue` | Replaced by `DesignSidebar.vue` |
| `Job.vue` | Replaced by data-table rows |
| `Group.vue` | Replaced by data-table rows |
| `Client1.vue` | Replaced by data-table rows |
| `DesignPrototype.vue` | Design routes removed |
| `hero-card.scss` | Unused styles |
