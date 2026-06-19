# Remex Endpoint — Architecture & Operation

## Purpose

The **Remex Endpoint** is an edge client that runs on remote machines to execute scheduled jobs. It operates in an **offline-first** manner, caching jobs locally and executing them independently of network connectivity. Execution results are stored locally first, then batch-synced to the core SurrealDB when a connection is available.

The endpoint is designed for environments where:
- Network connectivity is intermittent or unreliable
- Jobs must continue running even when disconnected from the central server
- Execution results must be reliably reported back when connectivity is restored

## System Architecture

```
┌─────────────────┐       TCP (encrypted)       ┌─────────────────┐
│   Remex Server  │◄──────────────────────────►│   Remex Endpoint│
│   (central hub) │                            │   (edge client)  │
└────────┬────────┘                            └────────┬─────────┘
         │                                              │
         │ WebSocket                                    │ Local SurrealKv DB
         ▼                                              │ (offline cache)
┌─────────────────┐                            ┌────────▼─────────┐
│  Core SurrealDB │◄─────── WebSocket ────────►│  Remote DB Conn  │
│  (cloud)        │                            │  (when connected)│
└─────────────────┘                            └──────────────────┘
```

### Key Components

| Component | Purpose |
|-----------|---------|
| **Server Message Loop** | Maintains TCP connection to Remex Server. Handles authentication, receives bearer tokens and remote DB URLs. |
| **Remote DB Connection** | WebSocket connection to core SurrealDB. Used for fetching jobs, live queries, and syncing execution results. |
| **Local Database** | SurrealKv embedded database. Stores session data, job cache, execution cache, and last_action tracking. |
| **Job Scheduler Loop** | Priority queue (BinaryHeap) that schedules and dispatches job executions based on timing. |
| **Job Monitor** | Live query watcher on the remote `job` and `group` tables. Detects creates, updates, and deletes in real-time. |
| **Execution Sync Loop** | Background task that pushes unsynced local executions to the remote DB every 30 seconds. |

## Connection & Startup Flow

1. **Local DB initialization** — Connects to `endpoint.db` (SurrealKv), runs migrations
2. **Session loading** — Loads or creates a session record (hardware hash, hostname)
3. **Server connection** — Spawns `server_msg_loop` to connect to TCP server
4. **Authentication** — Signs up or signs in with the server, receives bearer token + remote DB URL
5. **Remote DB connection** — Connects to core SurrealDB via WebSocket using bearer token
6. **Job sync** — Fetches all assigned jobs from remote, caches them locally
7. **Live queries** — Sets up live query streams on `job` and `group` tables
8. **Background tasks** — Spawns scheduler loop, sync loop, and monitor

## Job Lifecycle

### Discovery

Jobs are discovered through two paths:
- **Initial sync** — On first remote connection, all assigned jobs are fetched and cached locally
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
3. **Sync is asynchronous** — The `execution_sync_loop` runs every 30 seconds, pushing unsynced executions to remote
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
| `secret` | `option<string>` | Server authentication secret |
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

### Server Connection (TCP)
- Encrypted TCP socket with AES-GCM
- Packet-based protocol with 128-byte fixed-size packets
- Messages classified by prefix: `0` = command, `1` = secret, other = log
- Handles signup, signin, ping/pong, and disconnect reasons

### Remote DB Connection (WebSocket)
- SurrealDB WebSocket protocol
- Authenticated via `endpoint` BEARER access with token from server
- Used for: job/group queries, live queries, execution sync

## Key Design Decisions

### Offline-First
Executions are always written locally first. No execution data is lost if the network drops. The sync loop handles eventual consistency.

### Completion Tracking via JobCache
Rather than scanning execution history to determine if a job needs re-running, the `completed` flag on `JobCache` provides O(1) lookup. This is reset only when the job is actually updated (detected via `updated_at` comparison during sync, or via live query `Action::Update`).

### Pre-Mark Sync Strategy
Marking `synced = true` before pushing to remote prevents duplicate syncs. The only risk is a local DB write failure, which is rare and handled by the next sync cycle.

### Time-Based Cleanup
Synced executions are kept for 7 days to support duplicate prevention and debugging. The 6-hour cleanup interval (tracked via `last_action`) prevents unbounded storage growth.

### Staggered Maintenance Tasks
Critical tasks (`monitor_jobs`, `job_scheduler_loop`) start immediately on boot. Non-essential maintenance tasks (cleanup) check `last_action` before running, preventing all tasks from firing simultaneously on startup.
