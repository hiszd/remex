# Remex Endpoint — Architecture & Operation

## Purpose

The **Remex Endpoint** is an edge client that runs on remote machines to execute scheduled jobs. It operates in an **offline-first** manner, caching jobs locally and executing them independently of network connectivity. Execution results are stored locally first, then batch-synced to the core SurrealDB when a connection is available.

The endpoint is designed for environments where:
- Network connectivity is intermittent or unreliable
- Jobs must continue running even when disconnected from the core database
- Execution results must be reliably reported back when connectivity is restored

## System Architecture

```
┌─────────────────┐       WebSocket             ┌─────────────────┐
│  Core SurrealDB │◄───────────────────────────►│   Remex Endpoint│
│  (cloud)        │   (direct DB auth)          │   (edge client) │
└─────────────────┘                             └────────┬────────┘
                                                         │
                                                ┌────────▼─────────┐
                                                │  Local SurrealKV │
                                                │  (offline cache) │
                                                └──────────────────┘
```

> **Note:** A legacy `Remex Server` TCP path still exists in the `server/` crate, but current endpoints do **not** connect to it. Endpoints authenticate directly to SurrealDB via `endpoint_access`.

### Key Components

| Component | Actor / File | Purpose |
|-----------|--------------|---------|
| **SchedulerActor** | `async_tasks/jobs/scheduler.rs` | Priority queue (`BinaryHeap`) that schedules and dispatches job executions based on timing. |
| **RemoteDbActor** | `async_tasks/remote_db.rs` | Owns the single remote `Surreal<Any>` connection. Handles the direct DB auth loop, heartbeat, combined LIVE SELECT + initial sync, and execution push to the remote database. |
| **LocalDbActor** | `async_tasks/local_db.rs` | Owns the local SurrealKV handle. Handles session management, local cache operations, the execution sync tick, and the cleanup tick. |

All three actors are started under Actix `Supervisor`, so they restart automatically on panic.

## Connection & Startup Flow

1. **Local DB initialization** — Connects to `endpoint.db` (SurrealKV), runs migrations
2. **Start LocalDbActor** — Loads session from the local `session` table; schedules periodic ticks
3. **Start SchedulerActor** — Creates the job queue and wires it to LocalDbActor for `RecordExecution`
4. **Start RemoteDbActor** — Given `db_url`, optional `enrollment_token`, and `hardware_hash`
5. **Wire RemoteDbActor to LocalDbActor** — So unsynced executions can be pushed upstream
6. **Direct DB authentication** — RemoteDbActor's `connection_loop` signs in with stored credentials if available (takes precedence over enrollment token), or signs up with enrollment token via `endpoint_access`. Signup authenticates directly — no separate re-signin is needed.
7. **On successful auth** — RemoteDbActor runs `supervise_connection` which first runs `initial_sync` then spawns `heartbeat_loop`, `live_select_job`, and `live_select_group`
8. **LIVE SELECT + initial sync** — Loads cached jobs, runs `initial_sync` (fetches groups and jobs from remote), then subscribes to live notifications on `job` and `group`
9. **Background tasks** — Scheduler loop, execution sync tick, and cleanup tick run continuously

## Internal Tasks

### RemoteDbActor (spawned after successful auth)

| Task | Interval / Trigger | Purpose |
|------|-------------------|---------|
| `connection_loop` | Continuous | Loads session (retries with backoff from LocalDbActor), connects to remote, performs SIGNIN (stored creds take precedence) or SIGNUP via `endpoint_access`. No re-signin after signup. |
| `heartbeat_loop` | Every 60s | `UPDATE client SET last_seen = time::now()` |
| `supervise_connection` | One-shot after auth | Runs `initial_sync` (groups + jobs), then spawns `live_select_job` and `live_select_group`, re-syncs jobs on group changes |
| `PushExecution` handler | On demand | Sends a queued execution to the remote DB (or queues it while disconnected) |

### LocalDbActor (scheduled on startup and restart)

| Task | Interval | Purpose |
|------|----------|---------|
| `ExecutionSyncTick` | Every 30s | Find unsynced executions, send `PushExecution` to RemoteDbActor |
| `CleanupTick` | Every 30s (throttled to 6h) | Delete old synced executions and stale `last_action` records |
| Session load | On start / restart | Read `session` table into memory |

## Job Lifecycle

### Discovery

Jobs are discovered through two paths:
- **Initial sync** — On first remote connection, `spawn_live_select_tasks` calls `full_sync`, fetching all assigned jobs and caching them locally
- **Live queries** — Real-time notifications when jobs are created, updated, or deleted on the remote

### Scheduling

Jobs are scheduled based on their `job_type`:
- **Instant** — Executes immediately upon discovery
- **Scheduled** — Executes at a specific datetime
- **Recurring** — Executes at a datetime with a repeat interval (recurrence handling is pending)

The scheduler uses a `BinaryHeap` ordered by execution time. Jobs are popped from the heap when their execution time arrives and spawned as async tasks.

### Execution

When a job is dispatched for execution:

1. **Duplicate prevention** — `should_skip_job()` checks the local `JobCache` for `completed = true`. If the job is already completed and hasn't been updated, it's skipped.
2. **Execution record created** — An `ExecutionCache` entry is created with status `Running`
3. **Shell validation** — The job's `job_shell` is validated as an existing executable
4. **Command execution** — The command is run via `shell -c command` with optional timeout
5. **Status update** — The execution record is updated to `Completed`, `Failed`, or `TimedOut`
6. **Job completion** — If successful, the `JobCache` entry is marked `completed = true`

### Completion Tracking

The `completed` field on `JobCache` tracks whether a job has been successfully executed by this endpoint:

| Event | `completed` status |
|-------|-------------------|
| New job discovered | `false` |
| Job synced from remote (unchanged) | Preserved (no reset) |
| Job synced from remote (updated) | `false` (needs re-run) |
| Job updated via live query | `false` (needs re-run) |
| Job executed successfully | `true` |
| Job execution failed | `false` (will retry) |

This prevents redundant executions when the endpoint reconnects after being offline — if a job was already completed and hasn't changed, it won't be re-run.

## Offline-First Design

### How It Works

1. **Jobs are always cached locally** — The `JobCache` table stores all assigned jobs with their full metadata
2. **Executions are always written locally first** — The `ExecutionCache` table stores every execution with a `synced` flag
3. **Sync is asynchronous** — The `ExecutionSyncTick` runs every 30 seconds, pushing unsynced executions to remote
4. **No data loss on disconnect** — If the remote connection drops, executions are still recorded locally and synced when connectivity returns

### Sync Reliability

The execution sync uses a **pre-mark strategy** to prevent duplicate syncs:
1. Mark `synced = true` in local DB **before** pushing to remote
2. Push to remote DB
3. If push fails, revert `synced = false` so it will be retried next cycle

This eliminates the race condition where a slow remote push could cause the same execution to be synced twice.

### Cleanup

- **Synced executions** older than 7 days are deleted every 6 hours (tracked via `last_action` table)
- **`last_action` records** older than 72 hours are auto-purged during each cleanup cycle
- Cleanup only runs if it hasn't run in the past 6 hours, preventing redundant work on boot

## Database Tables (Local)

### `session` (namespace: `remex`, database: `endpoint`)
Stores the endpoint's identity and connection state.

| Field | Type | Description |
|-------|------|-------------|
| `client_id` | `option<string>` | Remote client record ID |
| `client_name` | `string` | Hostname |
| `hardware_hash` | `string` | Machine UID |
| `db_addr` | `option<string>` | Remote DB WebSocket URL |
| `tkn` | `option<object>` | Bearer token from server |
| `secret` | `option<string>` | Authentication secret |
| `groups` | `array<record<group>>` | Assigned group IDs |

### `job` (namespace: `remex`, database: `remex`) — JobCache
Cached copy of remote jobs for offline operation.

| Field | Type | Description |
|-------|------|-------------|
| `job_id` | `string` | Remote job record ID |
| `job_info` | `object` | Full job data (name, command, type, etc.) |
| `completed` | `bool` | Whether this endpoint has successfully executed this job |

### `execution` (namespace: `remex`, database: `remex`) — ExecutionCache
Local execution records, synced to remote when connected.

| Field | Type | Description |
|-------|------|-------------|
| `execution_id` | `string` | Unique execution identifier |
| `execution_info` | `object` | Full execution data (status, output, timestamps) |
| `synced` | `bool` | Whether this execution has been pushed to remote |

### `last_action` (namespace: `remex`, database: `endpoint`)
Tracks when periodic maintenance tasks last ran.

| Field | Type | Description |
|-------|------|-------------|
| `task_name` | `string` | Task identifier (e.g., "cleanup_executions") |
| `last_run` | `datetime` | When the task last ran |

Records older than 72 hours are automatically purged.

## Communication Protocol

### Remote DB Connection (WebSocket)
- SurrealDB WebSocket protocol
- Authenticated directly via `endpoint_access` RECORD access (enrollment token for SIGNUP, `hardware_hash` + `secret` for SIGNIN; `DURATION FOR TOKEN 1d`)
- Used for: job/group queries, live queries, execution sync

### Legacy Server Connection (TCP)
- Encrypted TCP socket with AES-GCM, packet-based 128-byte protocol
- Implemented in the transitional `server/` crate
- **Not used by current endpoints**; retained for migration/legacy scenarios only

## Key Design Decisions

### Direct Database Authentication
Endpoints authenticate directly to SurrealDB via `endpoint_access` instead of going through a central TCP server. The legacy TCP server path is retained only as a transitional utility.

### Three-Actor Supervised Architecture
Responsibilities are split into `SchedulerActor`, `RemoteDbActor`, and `LocalDbActor`, each started under an Actix `Supervisor`. This isolates failures: a panic in one actor restarts only that actor without bringing down the whole endpoint.

### Offline-First
Executions are always written locally first. No execution data is lost if the network drops. The sync loop handles eventual consistency.

### Completion Tracking via JobCache
Rather than scanning execution history to determine if a job needs re-running, the `completed` flag on `JobCache` provides O(1) lookup. This is reset only when the job is actually updated (detected via `updated_at` comparison during sync, or via live query `Action::Update`).

### Pre-Mark Sync Strategy
Marking `synced = true` before pushing to remote prevents duplicate syncs. The only risk is a local DB write failure, which is rare and handled by the next sync cycle.

### Time-Based Cleanup
Synced executions are kept for 7 days to support duplicate prevention and debugging. The 6-hour cleanup interval (tracked via `last_action`) prevents unbounded storage growth.

### Staggered Maintenance Tasks
Critical tasks (scheduler, live queries) start immediately on boot. Non-essential maintenance tasks (cleanup) check `last_action` before running, preventing all tasks from firing simultaneously on startup.
