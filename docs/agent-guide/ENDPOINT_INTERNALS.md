# Endpoint Internals

## Endpoint Architecture

### Actors

The endpoint uses **3 Actix actors** managed by `Supervisor`:

| Actor | File | Role |
|---|---|---|
| **SchedulerActor** | `async_tasks/jobs/scheduler.rs` | Job queue (BinaryHeap), spawns `execute_job` tasks |
| **RemoteDbActor** | `async_tasks/remote_db.rs` | Owns the single remote `Surreal<Any>` connection. Handles auth loop, heartbeat, LIVE SELECT, initial sync, execution push to remote |
| **LocalDbActor** | `async_tasks/local_db.rs` | Owns the local SurrealKV handle. Handles session management, local cache operations, execution sync tick, cleanup tick |

### Internal Tasks (spawned by actors)

**RemoteDbActor** spawns these tasks after successful auth:

| Task | Interval | Purpose |
|---|---|---|
| `connection_loop` | Reconnects / re-authenticates ~hourly | Drives the entire connection lifecycle: clear state, load/create local session, connect, signin (stored creds take precedence over enrollment token) or signup (enrollment token only), then enter `supervise_connection` |
| `heartbeat_loop` | Every 60s | `UPDATE client SET last_seen = time::now()` |
| `supervise_connection` | One-shot after auth | Runs `initial_sync` first, then spawns `live_select_job` and `live_select_group` tasks. Watches them with the re-auth timer |

**LocalDbActor** handles these periodic ticks on startup:

| Tick | Interval | Purpose |
|---|---|---|
| `ExecutionSyncTick` | Every 30s | Find unsynced executions, send `PushExecution` to RemoteDbActor |
| `CleanupTick` | Every 30s (throttled 6h) | Delete old synced executions from local cache |

### Message Flow

Messages defined in `endpoint/src/async_tasks.rs`:

| Message | Sender | Recipient | Purpose |
|---|---|---|---|
| `PushExecution { cache_id, execution }` | LocalDbActor | RemoteDbActor | Push an execution to remote DB |
| `MarkExecutionSynced { cache_id }` | RemoteDbActor | LocalDbActor | Mark local entry as synced |
| `CacheJob { job }` | RemoteDbActor | LocalDbActor | Cache a job locally (from LIVE SELECT) |
| `RecordExecution { result: ExecutionResult }` | SchedulerActor | LocalDbActor | Save execution result to local cache |
| `GetSession` | RemoteDbActor | LocalDbActor | Request stored session credentials |
| `SaveSession { client_id, secret }` | RemoteDbActor | LocalDbActor | Save session credentials after signup |
| `SetRemoteDbAddr(pub Addr<RemoteDbActor>)` | main wiring | LocalDbActor | Wire up the RemoteDbActor address so LocalDbActor can push unsynced executions |
| `ConnectionReady { db, client_id }` | legacy | legacy | Deprecated/legacy; still defined but not the active flow |
| `RemoteConnected { client_id }` | — | — | Defined but **never sent** |
| `RemoteDisconnected` | — | — | Defined but **never sent** |

Other messages in the design doc (`GetCachedJobs`, `GetCachedJob`, `SetCachedJobCompleted`, `ShouldSkipThrottle`, `RecordLastAction`) are **not implemented**.

### Seam Functions (testable, no actor dependency)

The `jobs` module (`endpoint/src/async_tasks/jobs/`) contains:

| Module | File | Key Exports |
|---|---|---|
| `scheduler` | `jobs/scheduler.rs` | `SchedulerActor`, `InjectJob` |
| `execution` | `jobs/execution.rs` | `ExecutionResult`, `execute_job()`, `should_skip_job()`, `mark_job_completed()`, `validate_shell()`, `run_command()` |
| `sync` | `jobs/sync.rs` | `sync_groups()`, `sync_job_to_cache()`, `push_unsynced_executions()` |

`JobQueueMessage` variants:

| Variant | Meaning |
|---|---|
| `Immediate { job, client_id }` | Execute the job right now |
| `Scheduled { job, execution_time, client_id }` | Execute the job at `execution_time` (an `Instant`) |
| `Remove { id }` | Remove a job from the scheduler queue by its `RecordId` |

### ExecutionResult and JobExecutor

**`ExecutionResult`** (`endpoint/src/async_tasks/jobs/execution.rs`) captures the outcome of a single job execution:

```rust
#[derive(Debug, Clone)]
pub struct ExecutionResult {
  pub output: String,
  pub exit_code: String,
  pub execution_start: surrealdb::types::Datetime,
  pub execution_end: Option<surrealdb::types::Datetime>,
  pub job_id: surrealdb::types::RecordId,
  pub client_id: surrealdb::types::RecordId,
  pub status: ExecutionStatus,
}
```

**`execute_job()`** signature uses `Option<ExecutionResult>` to distinguish two cases:

| Return value | Meaning |
|---|---|
| `Ok(None)` | Job was skipped (already completed recently — `should_skip_job()` returned `true`). No execution was created. |
| `Ok(Some(ExecutionResult { ... }))` | Job actually ran. The result has real output, exit_code, and timestamps. Scheduler should send it to LocalDbActor via `RecordExecution`. |
| `Err(e)` | Fatal error — local DB unavailable, invalid client_id, shell not found, command timeout. |

**`JobExecutor` trait** abstracts execution for testability:

```rust
#[async_trait]
pub trait JobExecutor: Send + Sync {
  async fn execute(&self, job: Job, client_id: &str)
    -> Result<Option<execution::ExecutionResult>, crate::Error>;
}
```

| Implementor | File | Context |
|---|---|---|
| `RealJobExecutor` | `jobs/mod.rs` | Production — delegates to `execution::execute_job()` |
| `MockJobExecutor` | `scheduler.rs` (tests) | Tests — records calls, returns a canned `Ok(Some(ExecutionResult { ... }))` |

The `JobExecutor` is injected into `SchedulerActor` via constructor:

```rust
pub struct SchedulerActor {
  executor: Arc<dyn JobExecutor>,
  // ...
}
```

**Scheduler flow** when a job fires:
1. Call `executor.execute(job, client_id).await`
2. If `Ok(Some(result))`, send `RecordExecution { result }` to LocalDbActor
3. If `Ok(None)`, log "job skipped" and do nothing
4. If `Err(e)`, log the error

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
- **ExecutionCache** is **one-way push**: created offline, marked `synced: false`, pushed to remote by `ExecutionSyncTick`
- **Session** and **LastAction** are purely local with no remote counterpart
