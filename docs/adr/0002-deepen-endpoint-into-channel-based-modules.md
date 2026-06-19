# Deepen endpoint into channel-based modules

The endpoint's `priority_queue.rs` (1147 lines) mixed scheduling, execution, sync, and monitoring in one module behind `Arc<Mutex<Context>>`. We split it into four modules communicating via `tokio::sync::mpsc` channels, plus a dedicated `DbConnector` module that owns the remote database connection lifecycle. The `Arc<Mutex<Context>>` is removed — each module owns only the state it needs.

- **Scheduler**: owns `BinaryHeap<ScheduledJob>`, receives `JobQueueMessage` from Monitor, spawns execution tasks.
- **Execution**: shell command runner. Writes `Running` status to local DB before executing, then updates with the final status. Owns `should_skip_job`.
- **Sync**: headless — polls local DB every 30s for unsynchronized execution records and pushes them to remote. Also handles cleanup and initial bulk sync via a channel message from Monitor.
- **Monitor**: owns the live query streams for job/group notifications and the `client_id`/`groups` session state. Delegates initial sync to Sync. Feeds job events to Scheduler.
- **DbConnector**: receives `(token, db_url)` from `server_msg_loop` via a one-shot channel, establishes the remote SurrealDB connection, and exposes a `watch::Sender<Option<Surreal<Any>>>` that Monitor and Sync subscribe to.

The design avoids shared locks entirely — no module accesses `Arc<Mutex<Context>>`.
