# Endpoint Execution Tracking & Sync Plan

## Overview

The endpoint executes jobs independently (offline-first) and reports execution results back to the core SurrealDB. Executions are always written locally first, then batch-synced to remote when connected.

## Current Gaps

| Gap | Description |
|-----|-------------|
| No remote sync | Executions stored locally are never sent to the core SurrealDB |
| Wrong client_id | Hardcoded to `client:self` instead of actual session client ID |
| No Running state | No execution record created with `Running` status before execution starts |
| Echo instead of shell | Commands are echoed, not executed via the job's shell |
| Migration not registered | `ExecutionCache::migrate()` is not called in `db::migrate()` |
| No sync background task | No loop to drain pending local executions to remote |
| No timeout handling | Jobs can run indefinitely |
| No shell validation | Shell path not validated before execution |

## Implementation Phases

### Phase 1: Fix the execution pipeline

**1.1. Wire up `ExecutionCache::migrate()`** in `endpoint/src/db.rs`

**1.2. Fix `client_id`** — pass the actual client ID from `Session` into the job execution context

**1.3. Two-phase execution status** — create execution with `Running` status before execution starts, update to `Completed`/`Failed`/`TimedOut` when done

**1.4. Fix command execution** — use `job.job_shell` with `-c` and `job.job_command` instead of `echo`

**1.5. Validate shell exists** — check that `job.job_shell` is a valid executable before running the job. If invalid, mark execution as `Failed` with appropriate error message.

### Phase 2: Add configurable timeout

**2.1. Add `timeout` field to `job` table** — `DEFINE FIELD timeout ON TABLE job TYPE option<duration>` (e.g., `5m`, `1h`). Default: no timeout.

**2.2. Update core `Job` struct** — add `timeout: Option<Duration>` field

**2.3. Update endpoint `JobCache` struct** — add matching field

**2.4. Update configurator** — add timeout input to job create/edit forms, display in job details

**2.5. Implement timeout in `execute_job`** — use `tokio::time::timeout()` to kill long-running commands. Mark execution as `TimedOut` if exceeded.

### Phase 3: Build the execution sync layer

**3.1. Add `synced` field to `ExecutionCache`** — boolean flag, `false` by default, set to `true` after successful remote push

**3.2. Create `execution_sync_loop`** — runs every 30 seconds:
   - Queries local executions where `synced = false`
   - Pushes each one to the remote DB via direct query
   - Marks `synced = true` on success
   - Handles offline gracefully (retries next interval)

**3.3. Spawn the sync loop** in `main.rs` alongside other background tasks

### Phase 4: Local execution lifecycle management

**4.1. Duplicate prevention** — before queuing a job, check local `ExecutionCache` for a recent execution of the same job. If one exists within the job's recurring interval (or already completed for one-shot), skip it.

**4.2. Cleanup of synced executions** — after an execution is synced, it's marked `synced = true`. A periodic cleanup task (every 6 hours, tracked via `last_action` table) deletes synced executions older than 7 days. This prevents unbounded local storage growth while keeping enough history for duplicate prevention. The `last_action` table auto-purges records older than 72 hours.

**4.3. No "closed" status needed** — the combination of `synced` flag + time-based cleanup handles the lifecycle cleanly. The scheduler's duplicate check uses a rolling window, so old synced executions being deleted doesn't cause re-runs.

## Data Flow

```
Job triggered (scheduler/live query)
  ↓
Check local ExecutionCache for recent execution (duplicate prevention, 5min window)
  ↓
Create Execution with status=Running in local DB
  ↓
Execute command (with shell validation + timeout)
  ↓
Update Execution status to Completed/Failed/TimedOut in local DB
  ↓
[Background: execution_sync_loop every 30s]
  ↓
Push unsynced executions to remote DB
  ↓
Mark local execution as synced=true
  ↓
[Background: cleanup task every 6h, tracked via last_action table]
  ↓
Delete synced executions older than 7 days
  ↓
[last_action table auto-purge every cleanup cycle]
  ↓
Delete last_action records older than 72 hours
```

## Key Design Decisions

- **Offline-first**: Executions always write locally first. No data loss if connection drops.
- **Two-phase status**: `Running` → `Completed`/`Failed`/`TimedOut`. Remote sees real-time status.
- **Batch sync**: 30-second interval, not immediate. Reduces connection chatter.
- **Time-based cleanup**: 7-day retention for synced executions. Prevents unbounded storage.
- **Rolling duplicate window**: Scheduler checks recent executions, not all history.
