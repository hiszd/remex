# RemoteDbActor + LocalDbActor Design

## Problem

The endpoint currently clones `Surreal<Any>` handles and passes them to multiple actors (HeartbeatActor, SyncActor, MonitorActor). This causes remote queries to hang because the cloned handles don't reliably share the WebSocket connection's `use_ns`/`use_db` state. Additionally, the architecture has too many actors with overlapping responsibilities.

## Solution

Replace the 5-actor architecture (DbConnectorActor, HeartbeatActor, SyncActor, MonitorActor, SchedulerActor) with a 3-actor architecture:

| Actor | Role |
|---|---|
| **SchedulerActor** | Job queue (BinaryHeap), spawns `execute_job` tasks |
| **RemoteDbActor** | Owns the single remote `Surreal<Any>` connection. Handles auth loop, heartbeat, LIVE SELECT, initial sync, execution push to remote |
| **LocalDbActor** | Owns the local SurrealKV handle. Handles session management, local cache operations, execution sync loop (finding unsynced and sending to RemoteDbActor), cleanup |

## Message Definitions

All messages live in `endpoint/src/async_tasks.rs`.

```rust
// ── Connection state (broadcast by RemoteDbActor) ──

/// Broadcast when the remote connection is established (after auth or re-auth).
#[derive(Message, Clone)]
#[rtype(result = "()")]
pub struct RemoteConnected {
    pub client_id: String,
}

/// Broadcast when the remote connection is lost.
#[derive(Message, Clone)]
#[rtype(result = "()")]
pub struct RemoteDisconnected;


// ── Remote queries (sent TO RemoteDbActor) ──

/// Push an unsynced execution to the remote database.
/// Sent by LocalDbActor's execution_sync_loop every 30s.
/// RemoteDbActor pushes to remote; on success sends MarkExecutionSynced to LocalDbActor.
/// On failure, logs and does nothing (LocalDbActor retries on next tick).
/// If not connected, queues the execution for later delivery.
#[derive(Message)]
#[rtype(result = "()")]
pub struct PushExecution {
    pub cache_id: String,
    pub execution: Execution,
}


// ── Local queries (sent TO LocalDbActor) ──

/// Mark a local execution cache entry as synced=true.
/// Sent by RemoteDbActor after a successful remote push.
#[derive(Message)]
#[rtype(result = "()")]
pub struct MarkExecutionSynced {
    pub cache_id: String,
    pub execution_info: Execution,
}

/// Get all cached jobs from local database.
#[derive(Message)]
#[rtype(result = "Result<Vec<JobCache>, DbError>")]
pub struct GetCachedJobs;

/// Get a single cached job by its remote job_id string.
#[derive(Message)]
#[rtype(result = "Result<Option<JobCache>, DbError>")]
pub struct GetCachedJob {
    pub job_id: String,
}

/// Upsert a job into the local cache.
#[derive(Message)]
#[rtype(result = "Result<(), DbError>")]
pub struct CacheJob {
    pub job: Job,
    pub completed: bool,
}

/// Mark a cached job as completed or incomplete.
#[derive(Message)]
#[rtype(result = "Result<(), DbError>")]
pub struct SetCachedJobCompleted {
    pub job_id: String,
    pub completed: bool,
}

/// Get the session (for auth).
#[derive(Message)]
#[rtype(result = "Result<Session, DbError>")]
pub struct GetSession;

/// Save session credentials after signup.
#[derive(Message)]
#[rtype(result = "Result<(), DbError>")]
pub struct SaveSession {
    pub session_id: String,
    pub client_id: String,
    pub secret: Option<String>,
}

/// Check if a cleanup task should be skipped (throttle).
#[derive(Message)]
#[rtype(result = "Result<bool, DbError>")]
pub struct ShouldSkipThrottle {
    pub task_name: String,
    pub interval_secs: u64,
}

/// Record that a task has run (for throttle tracking).
#[derive(Message)]
#[rtype(result = "Result<(), DbError>")]
pub struct RecordLastAction {
    pub task_name: String,
}
```

## RemoteDbActor

**File:** `endpoint/src/async_tasks/remote_db.rs`

### State

```rust
pub struct RemoteDbActor {
    // Config (immutable after construction)
    db_url: String,
    enrollment_token: Option<String>,

    // Connection state
    remote_db: Option<Surreal<Any>>,
    client_id: Option<String>,
    hardware_hash: String,
    connected: bool,

    // Pending pushes (queued while disconnected)
    pending_executions: Vec<PushExecution>,

    // References to other actors
    local_db_addr: Addr<LocalDbActor>,
    scheduler_addr: Addr<SchedulerActor>,

    // Subscribers for connection state
    connected_subscribers: Vec<Recipient<RemoteConnected>>,
}
```

### Startup (`started`)

1. Spawn `connection_loop()` as a tokio task
2. `connection_loop` runs the same auth logic as current `db_connector.rs`:
   - Load session from LocalDbActor (via message)
   - Try signin with stored credentials
   - If signin fails and enrollment token exists, try signup
   - On success: store `remote_db`, `client_id`, set `connected = true`
   - Send `RemoteConnected { client_id }` to all subscribers
   - Spawn internal tasks (heartbeat, LIVE SELECT, initial sync)
   - Schedule `ReauthTick` in 1 hour
   - Return (loop exits; actor holds the handle)

### Handlers

#### `Handler<PushExecution>`

```rust
fn handle(&mut self, msg: PushExecution) {
    if self.connected {
        let db = self.remote_db.clone().unwrap();
        let local_db = self.local_db_addr.clone();
        let cache_id = msg.cache_id.clone();
        tokio::spawn(async move {
            match db.query("CREATE execution CONTENT $data")
                .bind(("data", msg.execution))
                .await
            {
                Ok(result) if result.check().is_ok() => {
                    local_db.do_send(MarkExecutionSynced {
                        cache_id,
                        execution_info: msg.execution,
                    });
                }
                _ => {
                    tracing::warn!("Failed to push execution {cache_id} to remote");
                }
            }
        });
    } else {
        self.pending_executions.push(msg);
    }
}
```

#### `Handler<ReauthTick>` (internal)

```rust
fn handle(&mut self, _msg: ReauthTick, ctx: &mut Context<Self>) {
    // Get session from LocalDbActor
    // Try signin with stored credentials
    // On success: replace remote_db, broadcast RemoteConnected, drain pending, re-spawn tasks
    // On failure: broadcast RemoteDisconnected, spawn new connection_loop
}
```

### Internal Tasks (spawned after successful auth)

#### `heartbeat_loop`

```rust
async fn heartbeat_loop(remote_db: Surreal<Any>, client_id: RecordId) {
    loop {
        tokio::time::sleep(Duration::from_secs(60)).await;
        if let Err(e) = remote_db
            .query("UPDATE $id SET last_seen = time::now()")
            .bind(("id", client_id.clone()))
            .await
        {
            tracing::warn!("Heartbeat failed: {e}");
            // Don't break — keep trying
        }
    }
}
```

#### `live_select_job`

```rust
async fn live_select_job(
    remote_db: Surreal<Any>,
    local_db_addr: Addr<LocalDbActor>,
    scheduler_addr: Addr<SchedulerActor>,
    client_id: RecordId,
    groups: Vec<RecordId>,
) {
    let mut stream = match remote_db.select::<Vec<Job>>("job").live().await {
        Ok(s) => s,
        Err(e) => {
            tracing::error!("Failed to create job live query: {e}");
            return;
        }
    };

    while let Some(notification) = stream.next().await {
        match notification {
            Ok(n) => {
                // Check assignment
                let assigned = n.data.assignments.contains(&client_id)
                    || n.data.assignments.iter().any(|g| groups.contains(g));
                if !assigned {
                    continue;
                }

                match n.action {
                    Action::Create => {
                        // Cache locally
                        local_db_addr.send(CacheJob {
                            job: n.data.clone(),
                            completed: false,
                        }).await;

                        // Inject to scheduler if enabled
                        if n.data.enabled == Enabled::Enabled {
                            if let Some(exec_time) = calculate_execution_time(&n.data.job_type) {
                                scheduler_addr.send(InjectJob(Scheduled { job: n.data, exec_time, client_id }));
                            } else {
                                scheduler_addr.send(InjectJob(Immediate { job: n.data, client_id }));
                            }
                        }
                    }
                    Action::Update => {
                        // Update local cache
                        local_db_addr.send(CacheJob {
                            job: n.data.clone(),
                            completed: false,
                        }).await;

                        // Re-inject to scheduler if enabled
                        scheduler_addr.send(InjectJob(Remove { id: n.data.id.clone() }));
                        if n.data.enabled == Enabled::Enabled {
                            // ... inject logic same as Create
                        }
                    }
                    Action::Delete | Action::Killed => {
                        scheduler_addr.send(InjectJob(Remove { id: n.data.id }));
                    }
                }
            }
            Err(e) => {
                tracing::error!("Job stream error: {e}");
                break;
            }
        }
    }
}
```

#### `live_select_group`

```rust
async fn live_select_group(
    remote_db: Surreal<Any>,
    local_db_addr: Addr<LocalDbActor>,
    scheduler_addr: Addr<SchedulerActor>,
    client_id: RecordId,
) {
    let mut stream = match remote_db.select::<Vec<Group>>("group").live().await {
        Ok(s) => s,
        Err(e) => {
            tracing::error!("Failed to create group live query: {e}");
            return;
        }
    };

    let mut groups: Vec<RecordId> = Vec::new();

    while let Some(notification) = stream.next().await {
        match notification {
            Ok(n) => {
                let is_member = n.data.members.contains(&client_id);

                match n.action {
                    Action::Create if is_member => {
                        groups.push(n.data.id.clone());
                    }
                    Action::Update => {
                        groups.retain(|g| g != &n.data.id);
                        if is_member {
                            groups.push(n.data.id.clone());
                        }
                    }
                    Action::Delete | Action::Killed => {
                        groups.retain(|g| g != &n.data.id);
                    }
                    _ => continue,
                }

                // Re-sync jobs after group change
                if let Err(e) = sync_and_refill_queue(
                    &scheduler_addr, &client_id.to_sql(), &groups, &remote_db,
                ).await {
                    tracing::warn!("Group change re-sync failed: {e}");
                }
            }
            Err(e) => {
                tracing::error!("Group stream error: {e}");
                break;
            }
        }
    }
}
```

#### `initial_sync` (one-shot, runs before LIVE SELECT starts)

```rust
async fn initial_sync(
    remote_db: Surreal<Any>,
    local_db_addr: Addr<LocalDbActor>,
    scheduler_addr: Addr<SchedulerActor>,
    client_id: String,
) {
    // Fetch groups
    let groups = match sync_groups(&client_id, &remote_db).await {
        Ok(g) => g,
        Err(e) => {
            tracing::warn!("Initial sync: sync_groups failed: {e}");
            vec![]
        }
    };
    let group_ids: Vec<RecordId> = groups.iter().map(|g| g.id.clone()).collect();

    // Fetch jobs, cache locally, inject to scheduler
    if let Err(e) = sync_and_refill_queue(
        &scheduler_addr, &client_id, &group_ids, &remote_db,
    ).await {
        tracing::warn!("Initial sync: sync_and_refill_queue failed: {e}");
    }
}
```

#### `drain_pending_executions` (runs after re-auth)

```rust
fn drain_pending_executions(&mut self) {
    let pending = std::mem::take(&mut self.pending_executions);
    for msg in pending {
        self.handle(msg);  // re-processes through the handler (connected=true now)
    }
}
```

## LocalDbActor

**File:** `endpoint/src/async_tasks/local_db.rs`

### State

```rust
pub struct LocalDbActor {
    local_db: Surreal<Db>,  // clone of LOCAL_DB
    remote_db_addr: Addr<RemoteDbActor>,
}
```

### Startup (`started`)

1. Spawn `execution_sync_loop` as a tokio task
2. Spawn `cleanup_loop` as a tokio task

### Handlers

Each handler uses `get_local_remex()` or `get_local_endpoint()` to get a handle, then runs the query directly.

| Handler | Query |
|---|---|
| `GetCachedJobs` | `SELECT * FROM job` (remex.remex) |
| `GetCachedJob` | `SELECT * FROM job WHERE job_id = $id LIMIT 1` |
| `CacheJob` | `sync_job_to_cache()` seam function |
| `SetCachedJobCompleted` | `mark_job_completed()` or `mark_job_incomplete()` |
| `GetSession` | `SurrealSessionRepo::list()` → first session |
| `SaveSession` | `UPDATE session:id MERGE { client_id, secret }` |
| `ShouldSkipThrottle` | `LastAction::should_skip()` |
| `RecordLastAction` | `LastAction::record()` |
| `MarkExecutionSynced` | `UPDATE execution:id MERGE { synced: true }` |

### Internal Tasks

#### `execution_sync_loop`

```rust
async fn execution_sync_loop(
    local_db: Surreal<Db>,
    remote_db_addr: Addr<RemoteDbActor>,
) {
    loop {
        tokio::time::sleep(Duration::from_secs(30)).await;

        let unsynced: Vec<ExecutionCache> = match local_db
            .query("USE NS remex DB remex; SELECT * FROM execution WHERE synced = false;")
            .await
        {
            Ok(mut res) => res.take(1).unwrap_or_default(),
            Err(e) => {
                tracing::warn!("Failed to query unsynced executions: {e}");
                continue;
            }
        };

        for entry in unsynced {
            remote_db_addr.do_send(PushExecution {
                cache_id: entry.cache_id(),
                execution: entry.execution_info,
            });
        }
    }
}
```

#### `cleanup_loop`

```rust
async fn cleanup_loop(local_db: Surreal<Db>) {
    loop {
        tokio::time::sleep(Duration::from_secs(30)).await;

        let should_skip = LastAction::should_skip(
            &local_db, "cleanup_executions", 6 * 3600,
        ).await.unwrap_or(true);

        if !should_skip {
            if let Err(e) = local_db
                .query("USE NS remex DB remex; DELETE execution WHERE synced = true AND created_at < time::now() - 7d;")
                .await
            {
                tracing::warn!("Execution cleanup failed: {e}");
            } else {
                let _ = LastAction::record(&local_db, "cleanup_executions").await;
                let _ = LastAction::cleanup_old(&local_db).await;
            }
        }
    }
}
```

## main.rs (new wiring)

```rust
#[actix::main]
async fn main() -> Result<(), Error> {
    let args = Args::parse();
    init_logging(args.debug);

    db::LOCAL_DB.connect::<SurrealKv>("endpoint.db").await.unwrap();
    db::migrate(&db::LOCAL_DB).await.unwrap();

    // Start SchedulerActor
    let scheduler_addr = Supervisor::start(|_| SchedulerActor::new(Arc::new(RealJobExecutor)));

    // Start LocalDbActor (needs RemoteDbActor addr for execution push)
    // We start it first without the remote addr, then set it after RemoteDbActor is created
    let local_db_addr = Supervisor::start(|_| LocalDbActor::new());

    // Start RemoteDbActor (needs LocalDbActor addr + SchedulerActor addr)
    let remote_db_addr = Supervisor::start(|_| {
        RemoteDbActor::new(
            args.db_url,
            args.enrollment_token,
            local_db_addr.clone(),
            scheduler_addr.clone(),
        )
    });

    // Give LocalDbActor the RemoteDbActor addr (for PushExecution messages)
    local_db_addr.do_send(SetRemoteDbAddr(remote_db_addr.clone()));

    loop {
        tokio::time::sleep(Duration::from_secs(3600)).await;
    }
}
```

## Files to Create

| File | Contents |
|---|---|
| `endpoint/src/async_tasks/remote_db.rs` | RemoteDbActor (~350 lines) |
| `endpoint/src/async_tasks/local_db.rs` | LocalDbActor (~250 lines) |

## Files to Delete

| File | Reason |
|---|---|
| `endpoint/src/db_connector.rs` | Connection loop moved to RemoteDbActor |
| `endpoint/src/async_tasks/db_heartbeat.rs` | Heartbeat loop moved to RemoteDbActor internal task |
| `endpoint/src/async_tasks/jobs/monitor.rs` | LIVE SELECT + notification logic moved to RemoteDbActor internal tasks |
| `endpoint/src/async_tasks/jobs/sync.rs` (SyncActor only) | SyncActor removed; `full_sync`, `sync_groups`, `sync_and_refill_queue`, `sync_job_to_cache`, `push_unsynced_executions` seam functions stay |

## Files to Modify

| File | Change |
|---|---|
| `endpoint/src/async_tasks.rs` | Remove `ConnectionReady`, add new message types |
| `endpoint/src/async_tasks/jobs/mod.rs` | Remove `pub mod monitor;` and `pub mod sync;` (keep `pub mod execution;` and `pub mod scheduler;`) |
| `endpoint/src/async_tasks/jobs/sync.rs` | Remove `SyncActor` struct and its handlers; keep seam functions |
| `endpoint/src/main.rs` | Wire 3 actors instead of 6; remove `db_connector` module import |

## Files that Stay Unchanged

| File | Reason |
|---|---|
| `endpoint/src/db.rs` | `LOCAL_DB` static and `get_local_remex()`/`get_local_endpoint()` helpers still used by LocalDbActor and execute_job |
| `endpoint/src/db/remex.rs` | JobCache, ExecutionCache, repo structs unchanged |
| `endpoint/src/db/endpoint.rs` | Session structs unchanged |
| `endpoint/src/db/last_action.rs` | LastAction helpers unchanged |
| `endpoint/src/async_tasks/jobs/execution.rs` | execute_job, should_skip_job, mark_job_completed seam functions unchanged |
| `endpoint/src/async_tasks/jobs/scheduler.rs` | SchedulerActor unchanged |

## Test Impact

| Test file | Status |
|---|---|
| `db_connector::tests` | Move `create_new_session_with_repo` tests to `local_db.rs` |
| `heartbeat_tests` | Remove (heartbeat is now an internal task, tested via integration) |
| `sync_tests` | Keep — seam functions (`sync_job_to_cache`, `push_unsynced_executions`) are unchanged |
| `execution_tests` | Keep — seam functions (`should_skip_job`, `mark_job_completed`) are unchanged |
| `scheduler_tests` | Keep — SchedulerActor unchanged |
| `auth.rs` (integration) | Keep — tests the signup/signin flow which is unchanged |

## Migration Order

### Phase 1: Create both actors, wire minimally

1. Create `async_tasks/remote_db.rs` with RemoteDbActor + connection loop + heartbeat_loop
2. Create `async_tasks/local_db.rs` with LocalDbActor + execution_sync_loop + cleanup_loop
3. Update `async_tasks.rs` with new message types
4. Update `main.rs` to wire 3 actors
5. Delete `db_connector.rs`, `db_heartbeat.rs`
6. Move `create_new_session_with_repo` tests to `local_db.rs`

**Result:** Endpoint connects, authenticates, heartbeats, and pushes executions. No more hanging remote queries.

### Phase 2: Add LIVE SELECT + initial sync

7. Add `live_select_job` and `live_select_group` internal tasks to RemoteDbActor
8. Add `initial_sync` internal task to RemoteDbActor
9. Delete `monitor.rs`

**Result:** Endpoint monitors job/group changes and syncs jobs to scheduler.

### Phase 3: Clean up

10. Remove `SyncActor` from `sync.rs` (keep seam functions)
11. Remove `pub mod monitor;` and `pub mod sync;` from `jobs/mod.rs`
12. Clean up unused imports

**Result:** Clean 3-actor architecture.