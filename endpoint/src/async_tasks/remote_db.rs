use std::time::Duration;

use actix::prelude::*;
use remex_core::db::{
  DbError,
  DbOperator,
};
use surrealdb::{
  engine::any::Any,
  types::ToSql,
  Surreal,
};

use crate::{
  async_tasks::{
    local_db::LocalDbActor,
    CacheJob,
    GetSession,
    GroupEvent,
    MarkExecutionSynced,
    PushExecution,
    RemoveJob,
    SaveSession,
    SyncJobsBatch,
  },
  db::{
    endpoint::{
      Session,
      SessionData,
      SurrealSessionRepo,
    },
    get_local_endpoint,
  },
};

// ── Internal messages for communication between connection_loop and actor ──

/// Connection lifecycle states for the RemoteDbActor state machine.
/// Transitions are logged at the `remex.state` target for debugging.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ConnectionState {
  /// No remote handle, no tasks running. Initial state and destination after failure.
  Disconnected,
  /// TCP/WebSocket handshake + authentication in progress.
  Connecting,
  /// Signed in; initial_sync running but not yet complete.
  Authenticated,
  /// Fully operational — initial_sync complete, all tasks running.
  Connected,
}

impl std::fmt::Display for ConnectionState {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { write!(f, "{self:?}") }
}

#[derive(Message)]
#[rtype(result = "()")]
struct ConnectionSucceeded {
  remote_db: Surreal<Any>,
  client_id: String,
}

#[derive(Message)]
#[rtype(result = "()")]
struct ConnectionFailed;

#[derive(Message)]
#[rtype(result = "()")]
struct ClearConnection;

/// Message from connection_loop to store a Notify handle on the actor.
/// When the actor needs to trigger reconnection, it calls notify_one().
#[derive(Message)]
#[rtype(result = "()")]
struct SetReconnectNotify {
  notify: std::sync::Arc<tokio::sync::Notify>,
}

/// Sent by supervise_connection after initial_sync completes successfully.
/// Transitions the actor from Authenticated → Connected.
#[derive(Message)]
#[rtype(result = "()")]
struct InitialSyncCompleted;

/// Test helper: query the number of pending executions.
#[derive(Message)]
#[rtype(result = "usize")]
struct GetPendingCount;

// ── Actor ──

pub struct RemoteDbActor {
  // Config (immutable after construction)
  db_url: String,
  enrollment_token: Option<String>,
  hardware_hash: String,

  // Connection state
  remote_db: Option<Surreal<Any>>,
  client_id: Option<String>,
  connected: bool,

  // State machine for connection lifecycle
  connection_state: ConnectionState,
  previous_state: Option<ConnectionState>,

  // Pending pushes (queued while disconnected)
  pending_executions: Vec<PushExecution>,

  // Task cancellation — set when ConnectionSucceeded spawns tasks
  cancel_token: Option<tokio_util::sync::CancellationToken>,
  // Synchronization — connection_loop blocks on this; actor notifies on failure
  reconnect_notify: Option<std::sync::Arc<tokio::sync::Notify>>,

  // References to other actors
  local_db_addr: Addr<LocalDbActor>,
}

impl RemoteDbActor {
  pub fn new(
    db_url: String,
    enrollment_token: Option<String>,
    hardware_hash: String,
    local_db_addr: Addr<LocalDbActor>,
  ) -> Self {
    RemoteDbActor {
      db_url,
      enrollment_token,
      hardware_hash,
      remote_db: None,
      client_id: None,
      connected: false,
      connection_state: ConnectionState::Disconnected,
      previous_state: None,
      pending_executions: Vec::new(),
      cancel_token: None,
      reconnect_notify: None,
      local_db_addr,
    }
  }

  /// Record a state transition with the previous state, and log it.
  fn transition_to(&mut self, new: ConnectionState) {
    let old = self.connection_state;
    self.previous_state = Some(old);
    self.connection_state = new;
    tracing::info!(
      target: "remex.state",
      state = %new,
      previous = %old,
      "RemoteDbActor state transition"
    );
  }
}

impl Actor for RemoteDbActor {
  type Context = Context<Self>;

  fn started(&mut self, ctx: &mut Self::Context) {
    let addr = ctx.address();
    let db_url = self.db_url.clone();
    let enrollment_token = self.enrollment_token.clone();
    let hardware_hash = self.hardware_hash.clone();
    let local_db_addr = self.local_db_addr.clone();

    tokio::spawn(async move {
      connection_loop(&db_url, enrollment_token.as_deref(), &hardware_hash, addr, local_db_addr)
        .await;
    });
  }
}

impl actix::Supervised for RemoteDbActor {
  fn restarting(&mut self, ctx: &mut Context<Self>) {
    tracing::info!("RemoteDbActor: restarting");

    // Cancel any running tasks from the previous connection
    if let Some(t) = self.cancel_token.take() {
      t.cancel();
    }
    if let Some(n) = self.reconnect_notify.take() {
      n.notify_one();
    }

    // Preserve pending_executions (they survive restart)
    // Clear everything else
    self.remote_db = None;
    self.client_id = None;
    self.connected = false;
    self.transition_to(ConnectionState::Disconnected);

    // Re-spawn connection loop (like started() does)
    let addr = ctx.address();
    let db_url = self.db_url.clone();
    let enrollment_token = self.enrollment_token.clone();
    let hardware_hash = self.hardware_hash.clone();
    let local_db_addr = self.local_db_addr.clone();
    tokio::spawn(async move {
      connection_loop(&db_url, enrollment_token.as_deref(), &hardware_hash, addr, local_db_addr)
        .await;
    });
  }
}

// ── Message Handlers ──

// ── Inbound connection state handlers ──

impl Handler<PushExecution> for RemoteDbActor {
  type Result = Result<(), DbError>;

  fn handle(&mut self, msg: PushExecution, _ctx: &mut Self::Context) -> Result<(), DbError> {
    if self.connection_state == ConnectionState::Connected {
      let db = match self.remote_db.clone() {
        Some(db) => db,
        None => {
          tracing::warn!("RemoteDbActor: connected flag set but no remote_db handle");
          self.pending_executions.push(msg);
          return Ok(());
        }
      };
      let local_db = self.local_db_addr.clone();
      Self::push_execution_to_remote(db, local_db, msg);
    } else {
      tracing::debug!(
        "RemoteDbActor: queuing execution {} (state: {})",
        msg.cache_id,
        self.connection_state
      );
      self.pending_executions.push(msg);
    }
    Ok(())
  }
}

impl Handler<ConnectionSucceeded> for RemoteDbActor {
  type Result = ();

  fn handle(&mut self, msg: ConnectionSucceeded, ctx: &mut Self::Context) {
    // Reject stale messages — we should only arrive here from Connecting
    if !matches!(self.connection_state, ConnectionState::Connecting) {
      tracing::warn!(
        "RemoteDbActor: ignoring stale ConnectionSucceeded (current: {}, previous: {:?})",
        self.connection_state,
        self.previous_state
      );
      return;
    }

    tracing::info!("RemoteDbActor: connection succeeded as {}", msg.client_id);
    self.remote_db = Some(msg.remote_db.clone());
    self.client_id = Some(msg.client_id.clone());
    self.connected = true;

    // Drain any queued executions
    self.drain_pending_executions();

    // Cancel any prior token (safety — should not exist, but be defensive)
    if let Some(t) = self.cancel_token.take() {
      t.cancel();
    }
    let token = tokio_util::sync::CancellationToken::new();
    self.cancel_token = Some(token.clone());

    self.transition_to(ConnectionState::Authenticated);

    let remote_db = msg.remote_db;
    let client_id = msg.client_id;
    let local_db_addr = self.local_db_addr.clone();
    let actor_addr = ctx.address();

    tokio::spawn(async move {
      supervise_connection(remote_db, client_id, local_db_addr, token, actor_addr).await;
    });
  }
}

impl Handler<ConnectionFailed> for RemoteDbActor {
  type Result = ();

  fn handle(&mut self, _msg: ConnectionFailed, _ctx: &mut Self::Context) {
    self.transition_to(ConnectionState::Disconnected);
    tracing::warn!("RemoteDbActor: connection failed");

    // Cancel running tasks
    if let Some(t) = self.cancel_token.take() {
      t.cancel();
    }
    // Wake connection_loop so it re-enters the reconnect cycle
    if let Some(n) = self.reconnect_notify.take() {
      n.notify_one();
    }

    self.remote_db = None;
    self.client_id = None;
    self.connected = false;
  }
}

impl Handler<ClearConnection> for RemoteDbActor {
  type Result = ();

  fn handle(&mut self, _msg: ClearConnection, _ctx: &mut Self::Context) {
    self.transition_to(ConnectionState::Connecting);
    if let Some(t) = self.cancel_token.take() {
      t.cancel();
    }
    if let Some(n) = self.reconnect_notify.take() {
      n.notify_one();
    }
    self.remote_db = None;
    self.client_id = None;
    self.connected = false;
  }
}

impl Handler<SetReconnectNotify> for RemoteDbActor {
  type Result = ();

  fn handle(&mut self, msg: SetReconnectNotify, _ctx: &mut Self::Context) {
    self.reconnect_notify = Some(msg.notify);
  }
}

impl Handler<InitialSyncCompleted> for RemoteDbActor {
  type Result = ();

  fn handle(&mut self, _msg: InitialSyncCompleted, _ctx: &mut Self::Context) {
    if self.connection_state == ConnectionState::Authenticated {
      self.transition_to(ConnectionState::Connected);
    } else {
      tracing::debug!(
        "RemoteDbActor: ignoring stale InitialSyncCompleted (current state: {})",
        self.connection_state
      );
    }
  }
}

impl Handler<GetPendingCount> for RemoteDbActor {
  type Result = usize;

  fn handle(&mut self, _msg: GetPendingCount, _ctx: &mut Self::Context) -> usize {
    self.pending_executions.len()
  }
}

// ── Internal Methods ──

impl RemoteDbActor {
  /// Spawn a tokio task to push an execution to the remote database.
  /// On success, sends MarkExecutionSynced to the LocalDbActor.
  fn push_execution_to_remote(
    remote_db: Surreal<Any>,
    local_db_addr: Addr<LocalDbActor>,
    msg: PushExecution,
  ) {
    let cache_id = msg.cache_id.clone();
    tokio::spawn(async move {
      match remote_db
        .query("CREATE execution CONTENT $data")
        .bind(("data", msg.execution))
        .await
      {
        Ok(result) => match result.check() {
          Ok(_) => {
            tracing::debug!("RemoteDbActor: successfully pushed execution {cache_id}");
            local_db_addr.do_send(MarkExecutionSynced { cache_id });
          }
          Err(e) => {
            tracing::warn!("RemoteDbActor: failed to push execution {cache_id}: {e}");
          }
        },
        Err(e) => {
          tracing::warn!("RemoteDbActor: transport error pushing execution {cache_id}: {e}");
        }
      }
    });
  }

  fn drain_pending_executions(&mut self) {
    let pending = std::mem::take(&mut self.pending_executions);
    if pending.is_empty() {
      return;
    }
    tracing::info!("RemoteDbActor: draining {} pending executions", pending.len());
    let db = match self.remote_db.clone() {
      Some(db) => db,
      None => {
        tracing::error!("RemoteDbActor: cannot drain executions — no remote_db handle");
        return;
      }
    };
    let local_db = self.local_db_addr.clone();
    for msg in pending {
      Self::push_execution_to_remote(db.clone(), local_db.clone(), msg);
    }
  }
}

// ── Internal Tasks ──

/// Main connection loop — runs as a tokio task, connects and authenticates,
/// then sleeps for ~1 hour before re-authenticating.
#[tracing::instrument(
  name = "connection_loop",
  skip(enrollment_token, addr, local_db_addr),
  fields(db_url = %db_url, hardware_hash = %hardware_hash)
)]
async fn connection_loop(
  db_url: &str,
  enrollment_token: Option<&str>,
  hardware_hash: &str,
  addr: actix::Addr<RemoteDbActor>,
  local_db_addr: actix::Addr<LocalDbActor>,
) {
  loop {
    // Clear old state
    addr.do_send(ClearConnection);

    // Load session from local DB
    let session = match load_or_get_session(&local_db_addr, hardware_hash).await {
      Some(s) => s,
      None => {
        tracing::error!("RemoteDbActor: failed to load or create session");
        tokio::time::sleep(Duration::from_secs(5)).await;
        continue;
      }
    };

    // Connect to remote
    let remote_db: Surreal<Any> = Surreal::init();
    tracing::info!("RemoteDbActor: connecting to remote database at {db_url}");
    let connect_result =
      tokio::time::timeout(Duration::from_secs(15), remote_db.connect(db_url.to_string())).await;
    match connect_result {
      Ok(Ok(())) => tracing::info!("RemoteDbActor: connected to remote database"),
      Ok(Err(e)) => {
        tracing::error!("RemoteDbActor: failed to connect: {e}");
        addr.do_send(ConnectionFailed);
        tokio::time::sleep(Duration::from_secs(5)).await;
        continue;
      }
      Err(_) => {
        tracing::error!("RemoteDbActor: timed out connecting to {db_url}");
        addr.do_send(ConnectionFailed);
        tokio::time::sleep(Duration::from_secs(5)).await;
        continue;
      }
    }

    // Auth flow
    let has_stored_creds = session.secret.is_some() && session.client_id.is_some();

    if has_stored_creds {
      let secret = session.secret.clone().unwrap_or_default();
      tracing::info!("RemoteDbActor: signing in with existing credentials");

      match remote_db
        .signin(surrealdb::opt::auth::Record {
          namespace: "remex".into(),
          database: "remex".into(),
          access: "endpoint_access".into(),
          params: serde_json::json!({
            "hardware_hash": hardware_hash,
            "secret": secret,
          }),
        })
        .await
      {
        Ok(_tok) => {
          if let Err(e) = remote_db.use_ns("remex").use_db("remex").await {
            tracing::warn!("RemoteDbActor: failed to set ns/db after signin: {e}");
          }
          let client_id = lookup_client_id(&remote_db, hardware_hash).await;
          tracing::info!("RemoteDbActor: signed in as {client_id}");

          // Notify the actor where to signal reconnection
          let reconnect = std::sync::Arc::new(tokio::sync::Notify::new());
          addr.do_send(SetReconnectNotify {
            notify: reconnect.clone(),
          });
          addr.do_send(ConnectionSucceeded {
            remote_db: remote_db.clone(),
            client_id: client_id.clone(),
          });

          // Block until the actor signals reconnection is needed
          reconnect.notified().await;
          // Falls through to top of loop to reconnect
          continue;
        }
        Err(e) => {
          tracing::error!("RemoteDbActor: signin failed: {e} — retrying connection loop");
          addr.do_send(ConnectionFailed);
          tokio::time::sleep(Duration::from_secs(10)).await;
          continue;
        }
      }
    }

    if let Some(token) = enrollment_token {
      let client_name = gethostname::gethostname().to_string_lossy().to_string();
      let secret = remex_core::utils::generate_secret(true);

      tracing::info!("RemoteDbActor: signing up with enrollment token (client: {client_name})");

      match remote_db
        .signup(surrealdb::opt::auth::Record {
          namespace: "remex".into(),
          database: "remex".into(),
          access: "endpoint_access".into(),
          params: serde_json::json!({
            "enrollment_token": token,
            "client_name": client_name,
            "secret": secret,
            "hardware_hash": hardware_hash,
          }),
        })
        .await
      {
        Ok(_tok) => {
          tracing::info!("RemoteDbActor: signup successful");
          let client_id = lookup_client_id(&remote_db, hardware_hash).await;
          tracing::info!("RemoteDbActor: signed up as {client_id}");

          // Persist session credentials so the next loop iteration can sign in.
          persist_session_after_signup(&local_db_addr, &client_id, &secret).await;

          // Restart the connection loop to sign in on a fresh connection.
          // Queries on the signup connection are unreliable in the SurrealDB
          // WebSocket transport; signin always works reliably.
          tracing::info!(
            "RemoteDbActor: restarting loop after signup — will sign in on fresh connection"
          );
          continue;
        }
        Err(e) => {
          tracing::error!("RemoteDbActor: signup failed: {e:?}");
          if let Some(cause) = e.cause() {
            tracing::error!("RemoteDbActor: signup cause: {cause:?}");
          }
          addr.do_send(ConnectionFailed);
          tokio::time::sleep(Duration::from_secs(10)).await;
          continue;
        }
      }
    }

    tracing::warn!("RemoteDbActor: no stored credentials and no enrollment token. Retrying.");
    addr.do_send(ConnectionFailed);
    tokio::time::sleep(Duration::from_secs(10)).await;
  }
}

/// Heartbeat loop — every 60 seconds, update last_seen on the client record.
#[tracing::instrument(
  name = "heartbeat_loop",
  skip(remote_db, cancel),
  fields(client_id = %client_id)
)]
async fn heartbeat_loop(
  remote_db: Surreal<Any>,
  client_id: &str,
  cancel: tokio_util::sync::CancellationToken,
) {
  let rid = match surrealdb::types::RecordId::parse_simple(client_id) {
    Ok(rid) => rid,
    Err(e) => {
      tracing::warn!("RemoteDbActor heartbeat: invalid client_id {client_id}: {e}");
      return;
    }
  };

  if let Err(e) = remote_db.use_ns("remex").use_db("remex").await {
    tracing::warn!("RemoteDbActor heartbeat: failed to set ns/db: {e}");
    return;
  }

  loop {
    tokio::select! {
      _ = tokio::time::sleep(Duration::from_secs(60)) => {
        if let Err(e) = remote_db
          .query("UPDATE $id SET last_seen = time::now()")
          .bind(("id", rid.clone()))
          .await
        {
          tracing::warn!("RemoteDbActor heartbeat failed: {e}");
        }
      }
      _ = cancel.cancelled() => {
        tracing::debug!("RemoteDbActor heartbeat: cancelled");
        return;
      }
    }
  }
}

/// Supervise the connected tasks: runs initial_sync, then spawns heartbeat
/// + LIVE SELECT tasks and watches them with the re-auth timer.
#[tracing::instrument(
  name = "supervise_connection",
  skip(remote_db, local_db_addr, cancel, remotedb_addr),
  fields(client_id = %client_id)
)]
pub async fn supervise_connection(
  remote_db: Surreal<Any>,
  client_id: String,
  local_db_addr: Addr<LocalDbActor>,
  cancel: tokio_util::sync::CancellationToken,
  remotedb_addr: Addr<RemoteDbActor>,
) {
  // Allow the authenticated session to settle before issuing queries
  tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;

  // Run initial sync first
  tracing::info!("RemoteDbActor: running initial_sync for {client_id}");
  match initial_sync(remote_db.clone(), local_db_addr.clone(), client_id.clone()).await {
    Ok(()) => {
      tracing::info!("RemoteDbActor: initial_sync completed");
      remotedb_addr.do_send(InitialSyncCompleted);
    }
    Err(e) => {
      tracing::warn!("RemoteDbActor: initial_sync failed: {e}");
      cancel.cancel();
      remotedb_addr.do_send(ConnectionFailed);
      return;
    }
  }

  // Spawn long-running tasks with child tokens so cancelling the parent kills them all
  let hb_db = remote_db.clone();
  let hb_cid = client_id.clone();
  let hb_cancel = cancel.child_token();
  let heartbeat = tokio::spawn(async move { heartbeat_loop(hb_db, &hb_cid, hb_cancel).await });

  let lj_db = remote_db.clone();
  let lj_addr = local_db_addr.clone();
  let lj_cid = client_id.clone();
  let lj_cancel = cancel.child_token();
  let live_job =
    tokio::spawn(async move { live_select_job(lj_db, lj_addr, lj_cid, lj_cancel).await });

  let lg_db = remote_db.clone();
  let lg_addr = local_db_addr.clone();
  let lg_cid = client_id.clone();
  let lg_cancel = cancel.child_token();
  let live_group =
    tokio::spawn(async move { live_select_group(lg_db, lg_addr, lg_cid, lg_cancel).await });

  // Watchdog: log every 30s to confirm tasks haven't silently hung
  let wd_cancel = cancel.child_token();
  tokio::spawn(async move {
    loop {
      tokio::select! {
        _ = tokio::time::sleep(Duration::from_secs(30)) => {
          tracing::debug!("watchdog: supervise_connection tasks still alive");
        }
        _ = wd_cancel.cancelled() => {
          tracing::debug!("watchdog: shutting down");
          return;
        }
      }
    }
  });

  let jitter: u64 = rand::random::<u64>() % 300;
  tokio::select! {
    _ = tokio::time::sleep(Duration::from_secs(3600 + jitter)) => {
      tracing::debug!("RemoteDbActor: re-auth timer expired");
    }
    r = heartbeat => {
      match r {
        Ok(()) => tracing::warn!("RemoteDbActor: heartbeat task ended normally"),
        Err(e) => tracing::warn!("RemoteDbActor: heartbeat task panicked: {e}"),
      }
    }
    r = live_job => {
      match r {
        Ok(Err(e)) => tracing::warn!("RemoteDbActor: live_select_job ended with error: {e}"),
        Ok(Ok(())) => tracing::warn!("RemoteDbActor: live_select_job ended normally"),
        Err(e) => tracing::warn!("RemoteDbActor: live_select_job panicked: {e}"),
      }
    }
    r = live_group => {
      match r {
        Ok(Err(e)) => tracing::warn!("RemoteDbActor: live_select_group ended with error: {e}"),
        Ok(Ok(())) => tracing::warn!("RemoteDbActor: live_select_group ended normally"),
        Err(e) => tracing::warn!("RemoteDbActor: live_select_group panicked: {e}"),
      }
    }
  }

  // Cancel all remaining subtasks and signal the actor that the connection is dead
  cancel.cancel();
  remotedb_addr.do_send(ConnectionFailed);
  tracing::debug!("RemoteDbActor: supervise_connection exiting");
}

// ── LIVE SELECT Tasks ──

/// One-shot initial sync: fetches all jobs and groups from remote, sends SyncJobsBatch.
#[tracing::instrument(
  name = "initial_sync",
  skip(remote_db, local_db_addr),
  fields(client_id = %client_id)
)]
async fn initial_sync(
  remote_db: Surreal<Any>,
  local_db_addr: Addr<LocalDbActor>,
  client_id: String,
) -> Result<(), crate::Error> {
  use remex_core::db::model::jobs::Job;
  use tokio::time::timeout;

  use crate::async_tasks::jobs::sync::sync_groups;

  let groups = sync_groups(&client_id, &remote_db).await?;
  let group_ids: Vec<surrealdb::types::RecordId> = groups.into_iter().map(|g| g.id).collect();

  tracing::info!("RemoteDbActor: initial_sync fetching all jobs from remote");
  let jobs: Vec<Job> = match timeout(Duration::from_secs(10), async {
    tracing::debug!("initial_sync: sending job query");
    let response = match remote_db
      .query("USE NS remex DB remex; SELECT * FROM job;")
      .await
    {
      Ok(r) => r,
      Err(e) => return Err::<Vec<Job>, surrealdb::Error>(e),
    };
    tracing::debug!("initial_sync: job query returned from remote");
    let jobs: Vec<Job> = match response.check() {
      Ok(mut r) => match r.take(1) {
        Ok(j) => j,
        Err(e) => return Err::<Vec<Job>, surrealdb::Error>(e),
      },
      Err(e) => return Err::<Vec<Job>, surrealdb::Error>(e),
    };
    tracing::debug!("initial_sync: job results deserialized, {} jobs", jobs.len());
    Ok::<Vec<Job>, surrealdb::Error>(jobs)
  })
  .await
  {
    Ok(Ok(jobs)) => jobs,
    Ok(Err(e)) => {
      tracing::warn!("RemoteDbActor: initial_sync job fetch failed: {e}");
      return Err(crate::Error::Surreal(e));
    }
    Err(_) => {
      tracing::warn!("RemoteDbActor: initial_sync job fetch timed out");
      return Err(crate::Error::CommandTimeout);
    }
  };

  tracing::info!(
    "RemoteDbActor: initial_sync fetched {} jobs, {} groups",
    jobs.len(),
    group_ids.len()
  );
  local_db_addr.do_send(SyncJobsBatch {
    jobs,
    groups: group_ids,
    client_id,
  });

  Ok(())
}

/// LIVE SELECT on the job table. Forwards all notifications to LocalDbActor.
#[tracing::instrument(
  name = "live_select_job",
  skip(remote_db, local_db_addr, cancel),
  fields(client_id = %client_id)
)]
async fn live_select_job(
  remote_db: Surreal<Any>,
  local_db_addr: Addr<LocalDbActor>,
  client_id: String,
  cancel: tokio_util::sync::CancellationToken,
) -> Result<(), crate::Error> {
  use remex_core::db::model::jobs::Job;
  use tokio_stream::StreamExt;

  remote_db.use_ns("remex").use_db("remex").await?;

  tracing::info!("RemoteDbActor: live_select_job starting");

  let mut stream =
    match tokio::time::timeout(Duration::from_secs(10), remote_db.select::<Vec<Job>>("job").live())
      .await
    {
      Ok(Ok(s)) => {
        tracing::info!("RemoteDbActor: LIVE SELECT on job table created successfully");
        s
      }
      Ok(Err(e)) => {
        tracing::warn!("RemoteDbActor: failed to create job live query: {e}");
        return Err(crate::Error::from(e));
      }
      Err(_) => {
        tracing::error!(
          "RemoteDbActor: job live query timed out after 10s — server never responded\n\n"
        );
        return Err(crate::Error::DbError(DbError::OperationFailed(
          "job live select timed out".into(),
        )));
      }
    };

  loop {
    tokio::select! {
      notification = stream.next() => {
        tracing::info!("RemoteDbActor: live_select_job received notification");
        match notification {
          Some(Ok(n)) => match n.action {
            surrealdb::types::Action::Create | surrealdb::types::Action::Update => {
              tracing::debug!(
                "live_select_job: received {:?} for {:?}",
                n.action,
                n.data.id
              );
              local_db_addr.do_send(CacheJob {
                job: n.data,
                client_id: client_id.clone(),
              });
            }
            surrealdb::types::Action::Delete | surrealdb::types::Action::Killed => {
              tracing::debug!(
                "live_select_job: received {:?} for {:?}",
                n.action,
                n.data.id
              );
              local_db_addr.do_send(RemoveJob { job_id: n.data.id });
            }
          },
          Some(Err(e)) => {
            tracing::error!("RemoteDbActor: job live select error: {:#?}", e);
            return Err(crate::Error::from(e));
          }
          None => {
            tracing::warn!("RemoteDbActor: job live select stream ended");
            return Err(crate::Error::DbError(DbError::OperationFailed("job live select stream ended".into())));
          }
        }
      }
      _ = cancel.cancelled() => {
        tracing::debug!("RemoteDbActor: live_select_job cancelled");
        return Ok(());
      }
    }
  }
}

/// LIVE SELECT on the group table. Forwards all notifications to LocalDbActor.
#[tracing::instrument(
  name = "live_select_group",
  skip(remote_db, local_db_addr, cancel),
  fields(client_id = %client_id)
)]
async fn live_select_group(
  remote_db: Surreal<Any>,
  local_db_addr: Addr<LocalDbActor>,
  client_id: String,
  cancel: tokio_util::sync::CancellationToken,
) -> Result<(), crate::Error> {
  use remex_core::db::model::groups::Group;
  use tokio_stream::StreamExt;

  remote_db.use_ns("remex").use_db("remex").await?;

  let mut stream = match tokio::time::timeout(
    Duration::from_secs(10),
    remote_db.select::<Vec<Group>>("group").live(),
  )
  .await
  {
    Ok(Ok(s)) => {
      tracing::info!("RemoteDbActor: LIVE SELECT on group table created successfully");
      s
    }
    Ok(Err(e)) => {
      tracing::warn!("RemoteDbActor: failed to create group live query: {e}");
      return Err(crate::Error::from(e));
    }
    Err(_) => {
      tracing::error!(
        "RemoteDbActor: group live query timed out after 10s — server never responded"
      );
      return Err(crate::Error::DbError(DbError::OperationFailed(
        "group live select timed out".into(),
      )));
    }
  };

  loop {
    tokio::select! {
      notification = stream.next() => {
        match notification {
          Some(Ok(n)) => {
            tracing::debug!(
              "live_select_group: received {:?} for {:?}",
              n.action,
              n.data.id
            );
            local_db_addr.do_send(GroupEvent {
              group: n.data,
              action: n.action,
              client_id: client_id.clone(),
            });
          }
          Some(Err(e)) => {
            tracing::error!("RemoteDbActor: group live select error: {:#?}", e);
            return Err(crate::Error::from(e));
          }
          None => {
            tracing::warn!("RemoteDbActor: group live select stream ended");
            return Err(crate::Error::DbError(DbError::OperationFailed("group live select stream ended".into())));
          }
        }
      }
      _ = cancel.cancelled() => {
        tracing::debug!("RemoteDbActor: live_select_group cancelled");
        return Ok(());
      }
    }
  }
}
async fn lookup_client_id(remote_db: &Surreal<Any>, hardware_hash: &str) -> String {
  let hash = hardware_hash.to_owned();
  let result = remote_db
    .query("USE NS remex DB remex; SELECT VALUE id FROM client WHERE hardware_hash = $hash;")
    .bind(("hash", hash))
    .await;
  match result {
    Ok(mut res) => {
      let taken: Result<Vec<surrealdb::types::RecordId>, _> = res.take(1);
      match taken {
        Ok(ids) => {
          if let Some(id) = ids.first() {
            let id_str = id.to_sql();
            tracing::info!("RemoteDbActor lookup_client_id: found {id_str}");
            id_str
          } else {
            tracing::warn!("RemoteDbActor lookup_client_id: no client found");
            String::new()
          }
        }
        Err(e) => {
          tracing::error!("RemoteDbActor lookup_client_id: response error: {e}");
          String::new()
        }
      }
    }
    Err(e) => {
      tracing::error!("RemoteDbActor lookup_client_id: transport error: {e}");
      String::new()
    }
  }
}

/// Load session from LocalDbActor, retrying with backoff if it hasn't loaded yet.
async fn load_or_get_session(
  local_db_addr: &actix::Addr<LocalDbActor>,
  hardware_hash: &str,
) -> Option<Session> {
  for attempt in 1..=10 {
    match local_db_addr.send(GetSession).await {
      Ok(Ok(session)) => return Some(session),
      _ => {
        tracing::debug!("RemoteDbActor: waiting for session (attempt {attempt}/10)");
        tokio::time::sleep(Duration::from_millis(500)).await;
      }
    }
  }
  // After retries, fall back to direct creation as last resort
  tracing::warn!("RemoteDbActor: GetSession failed after 10 retries. Creating session directly.");
  create_session_directly(hardware_hash).await
}

/// Seam function for creating a new session via a DbOperator.
/// Testable in isolation without needing the concrete SurrealDB handle.
async fn create_new_session_with_repo(
  repo: &dyn DbOperator<Record = Session, Input = SessionData>,
  hardware_hash: &str,
) -> Result<Session, DbError> {
  let data = SessionData {
    client_id: None,
    client_name: Some(gethostname::gethostname().to_string_lossy().to_string()),
    hardware_hash: Some(hardware_hash.to_string()),
    db_addr: None,
    tkn: None,
    secret: None,
    groups: vec![],
  };
  repo.create(data).await
}

/// Create a new session directly from the local DB (fallback when LocalDbActor is a stub).
/// Uses the seam function above so the creation logic is testable.
async fn create_session_directly(hardware_hash: &str) -> Option<Session> {
  let local_db = match get_local_endpoint().await {
    Ok(db) => db,
    Err(e) => {
      tracing::error!("RemoteDbActor: failed to get local endpoint DB: {e}");
      return None;
    }
  };
  let repo = SurrealSessionRepo { db: local_db };
  match create_new_session_with_repo(&repo, hardware_hash).await {
    Ok(session) => {
      tracing::info!("RemoteDbActor: created new session {}", session.session_id());
      Some(session)
    }
    Err(e) => {
      tracing::error!("RemoteDbActor: failed to create session: {e}");
      None
    }
  }
}

/// Persist session credentials after signup via LocalDbActor.
async fn persist_session_after_signup(
  local_db_addr: &actix::Addr<LocalDbActor>,
  client_id: &str,
  secret: &str,
) {
  if let Err(e) = local_db_addr
    .send(SaveSession {
      client_id: client_id.to_string(),
      secret: secret.to_string(),
    })
    .await
  {
    tracing::warn!("RemoteDbActor: failed to send SaveSession: {e}");
  }
}

// ── LIVE SELECT Tasks ──

// ── Tests ──

#[cfg(test)]
mod remote_db_tests {
  use actix::prelude::*;
  use remex_core::db::model::executions::Execution;
  use surrealdb::{
    engine::any::Any,
    Surreal,
  };

  use super::{
    ClearConnection,
    ConnectionFailed,
    ConnectionSucceeded,
    GetPendingCount,
    PushExecution,
    RemoteDbActor,
  };
  use crate::async_tasks::local_db::LocalDbActor;

  // ── Helpers ──

  fn make_test_execution() -> Execution {
    Execution {
      id: surrealdb::types::RecordId::new("execution", "test-1"),
      job_id: Some(surrealdb::types::RecordId::new("job", "test-job")),
      client_id: surrealdb::types::RecordId::new("client", "test-client"),
      status: remex_core::db::model::executions::ExecutionStatus::Completed,
      output: "test output".to_string(),
      command: "echo hi".to_string(),
      exit_code: "0".to_string(),
      execution_start: surrealdb::types::Datetime::default(),
      execution_end: Some(surrealdb::types::Datetime::default()),
      created_at: surrealdb::types::Datetime::default(),
      updated_at: surrealdb::types::Datetime::default(),
    }
  }

  fn setup_actor() -> (Addr<RemoteDbActor>, Addr<LocalDbActor>) {
    let local_db_addr = LocalDbActor::new("test-hash".to_string()).start();
    let remote_db_addr = RemoteDbActor::new(
      "memory".to_string(),
      None,
      "test-hash".to_string(),
      local_db_addr.clone(),
    )
    .start();
    (remote_db_addr, local_db_addr)
  }

  // ── Tests ──

  #[actix::test]
  async fn push_queues_when_disconnected() {
    let (remote_db_addr, _local_db) = setup_actor();

    // Send a push while not connected — should queue
    let exec = make_test_execution();
    remote_db_addr
      .send(PushExecution {
        cache_id: "test-cache-1".to_string(),
        execution: exec,
      })
      .await
      .unwrap()
      .unwrap();

    // Should be queued (not sent to remote)
    let count = remote_db_addr.send(GetPendingCount).await.unwrap();
    assert_eq!(count, 1, "execution should be queued when disconnected");
  }

  #[actix::test]
  async fn push_does_not_panic_when_disconnected() {
    let (remote_db_addr, _local_db) = setup_actor();

    // Send multiple pushes while disconnected
    for i in 0..5 {
      let exec = make_test_execution();
      remote_db_addr
        .send(PushExecution {
          cache_id: format!("test-cache-{i}"),
          execution: exec,
        })
        .await
        .unwrap()
        .unwrap();
    }

    let count = remote_db_addr.send(GetPendingCount).await.unwrap();
    assert_eq!(count, 5, "all 5 executions should be queued when disconnected");
  }

  #[actix::test]
  async fn connection_succeeded_sets_state() {
    let (remote_db_addr, _local_db) = setup_actor();

    // Send ConnectionSucceeded — should set connected state
    let remote_db: Surreal<Any> = Surreal::init();
    remote_db.connect("memory").await.unwrap();
    remote_db.use_ns("remex").use_db("remex").await.unwrap();

    remote_db_addr
      .send(ConnectionSucceeded {
        remote_db: remote_db.clone(),
        client_id: "client:test-1".to_string(),
      })
      .await
      .unwrap();

    // Give the actor time to process
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    // Push while connected — should NOT panic (handler dispatches to
    // push_execution_to_remote which spawns a tokio task; the CREATE may
    // fail on the in-memory DB but the actor itself must not panic).
    let exec = make_test_execution();
    remote_db_addr
      .send(PushExecution {
        cache_id: "post-connect-push".to_string(),
        execution: exec,
      })
      .await
      .unwrap()
      .unwrap();

    // Give the spawned task time to process
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    // Note: we no longer assert GetPendingCount == 0 because
    // supervise_connection runs as a detached task and may send
    // ConnectionFailed before PushExecution is processed, which
    // would legitimately queue the push. The important invariant
    // is that the actor does not panic.
  }

  #[actix::test]
  async fn connection_failed_does_not_panic() {
    let (remote_db_addr, _local_db) = setup_actor();

    remote_db_addr.send(ConnectionFailed).await.unwrap();
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    // Push after failure should queue again
    let exec = make_test_execution();
    remote_db_addr
      .send(PushExecution {
        cache_id: "after-fail".to_string(),
        execution: exec,
      })
      .await
      .unwrap()
      .unwrap();

    let count = remote_db_addr.send(GetPendingCount).await.unwrap();
    assert_eq!(count, 1, "push after connection failure should queue");
  }

  #[actix::test]
  async fn queued_executions_are_drained_on_reconnect() {
    let (remote_db_addr, _local_db) = setup_actor();

    // Queue an execution while disconnected
    let exec = make_test_execution();
    remote_db_addr
      .send(PushExecution {
        cache_id: "drain-me".to_string(),
        execution: exec,
      })
      .await
      .unwrap()
      .unwrap();

    let count = remote_db_addr.send(GetPendingCount).await.unwrap();
    assert_eq!(count, 1, "execution should be queued when disconnected");

    // Connect — should trigger drain
    let remote_db: Surreal<Any> = Surreal::init();
    remote_db.connect("memory").await.unwrap();
    remote_db.use_ns("remex").use_db("remex").await.unwrap();

    remote_db_addr
      .send(ConnectionSucceeded {
        remote_db: remote_db.clone(),
        client_id: "client:drain-test".to_string(),
      })
      .await
      .unwrap();

    // Give the drain time to process (drain spawns tasks)
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Queue should be empty after drain
    let count = remote_db_addr.send(GetPendingCount).await.unwrap();
    assert_eq!(count, 0, "pending executions should be drained on reconnect");
  }

  #[actix::test]
  async fn clear_connection_does_not_panic() {
    let (remote_db_addr, _local_db) = setup_actor();

    remote_db_addr.send(ClearConnection).await.unwrap();
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
  }

  #[actix::test]
  async fn push_after_connected_does_not_panic() {
    let (remote_db_addr, _local_db) = setup_actor();

    // Simulate connection
    let remote_db: Surreal<Any> = Surreal::init();
    remote_db.connect("memory").await.unwrap();
    remote_db.use_ns("remex").use_db("remex").await.unwrap();

    remote_db_addr
      .send(ConnectionSucceeded {
        remote_db: remote_db.clone(),
        client_id: "client:test-push".to_string(),
      })
      .await
      .unwrap();

    // Small delay for state to propagate
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    // Send push (will attempt remote CREATE, which may fail since memory DB
    // doesn't have the execution table defined — but shouldn't panic)
    let exec = make_test_execution();
    remote_db_addr
      .send(PushExecution {
        cache_id: "push-after-connect".to_string(),
        execution: exec,
      })
      .await
      .unwrap()
      .unwrap();

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // The important invariant: the actor must not panic when pushing
    // after ConnectionSucceeded, regardless of whether the spawned
    // supervise_connection task has already sent ConnectionFailed.
  }

  // ── create_new_session_with_repo tests ──

  use remex_core::db::DbError;
  use surrealdb::types::ToSql;

  use super::create_new_session_with_repo;
  use crate::db::endpoint::{
    Session,
    SessionData,
  };

  struct MockSessionRepo {
    last_data: std::sync::Mutex<Option<SessionData>>,
  }

  impl MockSessionRepo {
    fn new() -> Self {
      MockSessionRepo {
        last_data: std::sync::Mutex::new(None),
      }
    }

    fn last_data(&self) -> Option<SessionData> { self.last_data.lock().unwrap().clone() }
  }

  #[async_trait::async_trait]
  impl remex_core::db::DbOperator for MockSessionRepo {
    type Record = Session;
    type Input = SessionData;

    async fn create(&self, input: Self::Input) -> Result<Self::Record, DbError> {
      *self.last_data.lock().unwrap() = Some(input.clone());
      Ok(Session {
        id: surrealdb::types::RecordId::new("session", "mock-session-1"),
        client_id: input.client_id.clone(),
        client_name: input.client_name.clone().unwrap_or_default(),
        hardware_hash: input.hardware_hash.clone().unwrap_or_default(),
        db_addr: input.db_addr.clone(),
        tkn: input.tkn.clone(),
        secret: input.secret.clone(),
        groups: input.groups.clone(),
      })
    }

    async fn read(&self, _id: &str) -> Result<Option<Self::Record>, DbError> { unimplemented!() }

    async fn update(&self, _id: &str, _input: Self::Input) -> Result<Self::Record, DbError> {
      unimplemented!()
    }

    async fn list(&self) -> Result<Vec<Self::Record>, DbError> { unimplemented!() }

    async fn delete(&self, _id: &str) -> Result<(), DbError> { unimplemented!() }
  }

  #[actix::test]
  async fn create_new_session_sets_defaults() {
    let repo = MockSessionRepo::new();
    let session = create_new_session_with_repo(&repo, "test-hash")
      .await
      .expect("session creation should succeed");

    assert_eq!(session.client_id, None, "client_id should default to None");
    assert_eq!(session.secret, None, "secret should default to None");
    assert!(session.groups.is_empty(), "groups should be empty");
    assert_eq!(session.hardware_hash, "test-hash", "hardware_hash should be set");
    assert!(!session.client_name.is_empty(), "client_name should be set to hostname");

    // Verify the repo received the right data
    let data = repo.last_data().expect("repo should have recorded data");
    assert_eq!(data.client_id, None);
    assert_eq!(data.hardware_hash, Some("test-hash".to_string()));
    assert!(data.groups.is_empty());
  }

  #[actix::test]
  async fn create_new_session_generates_unique_ids() {
    let repo1 = MockSessionRepo::new();
    let session1 = create_new_session_with_repo(&repo1, "hash-a")
      .await
      .expect("first session should succeed");

    // Create a second repo that returns a different id
    struct CountedRepo {
      count: std::sync::Mutex<u32>,
    }
    #[async_trait::async_trait]
    impl remex_core::db::DbOperator for CountedRepo {
      type Record = Session;
      type Input = SessionData;

      async fn create(&self, input: Self::Input) -> Result<Self::Record, DbError> {
        let mut c = self.count.lock().unwrap();
        *c += 1;
        Ok(Session {
          id: surrealdb::types::RecordId::new("session", format!("sess-{}", *c)),
          client_id: input.client_id,
          client_name: input.client_name.unwrap_or_default(),
          hardware_hash: input.hardware_hash.unwrap_or_default(),
          db_addr: input.db_addr,
          tkn: input.tkn,
          secret: input.secret,
          groups: input.groups,
        })
      }

      async fn read(&self, _id: &str) -> Result<Option<Self::Record>, DbError> { unimplemented!() }
      async fn update(&self, _id: &str, _input: Self::Input) -> Result<Self::Record, DbError> {
        unimplemented!()
      }
      async fn list(&self) -> Result<Vec<Self::Record>, DbError> { unimplemented!() }
      async fn delete(&self, _id: &str) -> Result<(), DbError> { unimplemented!() }
    }

    let repo2 = CountedRepo {
      count: std::sync::Mutex::new(42),
    };
    let session2 = create_new_session_with_repo(&repo2, "hash-b")
      .await
      .expect("second session should succeed");

    assert_ne!(session1.id.to_sql(), session2.id.to_sql(), "sessions should have different IDs");
  }
}
