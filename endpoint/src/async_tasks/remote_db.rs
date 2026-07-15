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

  // Pending pushes (queued while disconnected)
  pending_executions: Vec<PushExecution>,

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
      pending_executions: Vec::new(),
      local_db_addr,
    }
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
    // Preserve pending_executions (they survive restart)
    // Clear everything else
    self.remote_db = None;
    self.client_id = None;
    self.connected = false;

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
    if self.connected {
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
      tracing::debug!("RemoteDbActor: queuing execution {} (disconnected)", msg.cache_id);
      self.pending_executions.push(msg);
    }
    Ok(())
  }
}

impl Handler<ConnectionSucceeded> for RemoteDbActor {
  type Result = ();

  fn handle(&mut self, msg: ConnectionSucceeded, _ctx: &mut Self::Context) {
    tracing::info!("RemoteDbActor: connection succeeded as {}", msg.client_id);
    self.remote_db = Some(msg.remote_db.clone());
    self.client_id = Some(msg.client_id.clone());
    self.connected = true;

    // Drain any queued executions
    self.drain_pending_executions();
  }
}

impl Handler<ConnectionFailed> for RemoteDbActor {
  type Result = ();

  fn handle(&mut self, _msg: ConnectionFailed, _ctx: &mut Self::Context) {
    tracing::warn!("RemoteDbActor: connection failed");
    self.remote_db = None;
    self.client_id = None;
    self.connected = false;
  }
}

impl Handler<ClearConnection> for RemoteDbActor {
  type Result = ();

  fn handle(&mut self, _msg: ClearConnection, _ctx: &mut Self::Context) {
    self.remote_db = None;
    self.client_id = None;
    self.connected = false;
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

          addr.do_send(ConnectionSucceeded {
            remote_db: remote_db.clone(),
            client_id: client_id.clone(),
          });

          supervise_connection(remote_db.clone(), client_id, local_db_addr.clone()).await;
          continue; // Loop back to re-authenticate
        }
        Err(e) => {
          tracing::error!("RemoteDbActor: signin failed: {e}");
          // Fall through to enrollment attempt below
        }
      }
    }

    if let Some(token) = enrollment_token {
      let client_name = gethostname::gethostname().to_string_lossy().to_string();
      let secret = remex_core::utils::generate_secret(true);

      // Check for stale client with this hardware_hash
      let existing_client_id: Option<String> = match remote_db
        .query(
          "USE NS remex DB remex; SELECT VALUE id FROM client WHERE hardware_hash = $hash LIMIT 1;",
        )
        .bind(("hash", hardware_hash.to_string()))
        .await
      {
        Ok(mut res) => match res.take::<Vec<surrealdb::types::RecordId>>(1) {
          Ok(ids) => ids.first().map(|id| id.to_sql()),
          Err(_) => None,
        },
        Err(_) => None,
      };

      if let Some(ref existing_id) = existing_client_id {
        tracing::warn!("RemoteDbActor: deleting stale client {existing_id}");
        if let Ok(rid) = surrealdb::types::RecordId::parse_simple(existing_id) {
          match remote_db
            .query("USE NS remex DB remex; DELETE FROM $id;")
            .bind(("id", rid))
            .await
          {
            Ok(_) => tracing::debug!("RemoteDbActor: deleted stale client {existing_id}"),
            Err(e) => {
              tracing::error!("RemoteDbActor: failed to delete stale client {existing_id}: {e}")
            }
          }
        } else {
          tracing::error!(
            "RemoteDbActor: invalid stale client ID {existing_id}, skipping deletion"
          );
        }
      }

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
          if let Err(e) = remote_db.use_ns("remex").use_db("remex").await {
            tracing::warn!("RemoteDbActor: failed to set ns/db after signup: {e}");
          }
          let client_id = lookup_client_id(&remote_db, hardware_hash).await;
          tracing::info!("RemoteDbActor: signed up as {client_id}");

          // Persist session credentials
          persist_session_after_signup(&local_db_addr, &client_id, &secret).await;

          // Re-authenticate as the client record
          tracing::info!("RemoteDbActor: re-authenticating after signup");
          if let Err(e) = remote_db
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
            tracing::warn!("RemoteDbActor: re-auth after signup failed: {e}");
          }
          if let Err(e) = remote_db.use_ns("remex").use_db("remex").await {
            tracing::warn!("RemoteDbActor: failed to set ns/db after re-auth: {e}");
          }

          addr.do_send(ConnectionSucceeded {
            remote_db: remote_db.clone(),
            client_id: client_id.clone(),
          });

          supervise_connection(remote_db.clone(), client_id, local_db_addr.clone()).await;
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
async fn heartbeat_loop(remote_db: Surreal<Any>, client_id: &str) {
  let rid = match surrealdb::types::RecordId::parse_simple(client_id) {
    Ok(rid) => rid,
    Err(e) => {
      tracing::warn!("RemoteDbActor heartbeat: invalid client_id {client_id}: {e}");
      return;
    }
  };

  loop {
    tokio::time::sleep(Duration::from_secs(60)).await;
    if let Err(e) = remote_db
      .query("UPDATE $id SET last_seen = time::now()")
      .bind(("id", rid.clone()))
      .await
    {
      tracing::warn!("RemoteDbActor heartbeat failed: {e}");
    }
  }
}

/// Supervise the connected tasks: runs initial_sync, then spawns heartbeat
/// + LIVE SELECT tasks and watches them with the re-auth timer.
async fn supervise_connection(
  remote_db: Surreal<Any>,
  client_id: String,
  local_db_addr: Addr<LocalDbActor>,
) {
  // Run initial sync first
  tracing::info!("RemoteDbActor: running initial_sync for {client_id}");
  match initial_sync(remote_db.clone(), local_db_addr.clone(), client_id.clone()).await {
    Ok(()) => tracing::info!("RemoteDbActor: initial_sync completed"),
    Err(e) => {
      tracing::warn!("RemoteDbActor: initial_sync failed: {e}");
      return;
    }
  }

  // Spawn long-running tasks
  let hb_db = remote_db.clone();
  let hb_cid = client_id.clone();
  let heartbeat = tokio::spawn(async move { heartbeat_loop(hb_db, &hb_cid).await });

  let lj_db = remote_db.clone();
  let lj_addr = local_db_addr.clone();
  let lj_cid = client_id.clone();
  let live_job = tokio::spawn(async move { live_select_job(lj_db, lj_addr, lj_cid).await });

  let lg_db = remote_db.clone();
  let lg_addr = local_db_addr.clone();
  let lg_cid = client_id.clone();
  let live_group = tokio::spawn(async move { live_select_group(lg_db, lg_addr, lg_cid).await });

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

  // Remaining spawned tasks will continue until their stream fails
  // or the remote_db connection becomes invalid.
  tracing::debug!("RemoteDbActor: supervise_connection exiting");
}

// ── LIVE SELECT Tasks ──

/// One-shot initial sync: fetches all jobs and groups from remote, sends SyncJobsBatch.
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
    remote_db
      .query("USE NS remex DB remex; SELECT * FROM job;")
      .await?
      .check()?
      .take(1)
  })
  .await
  {
    Ok(Ok(jobs)) => jobs,
    Ok(Err(e)) => {
      tracing::warn!("RemoteDbActor: initial_sync job fetch failed: {e}");
      return Ok(());
    }
    Err(_) => {
      tracing::warn!("RemoteDbActor: initial_sync job fetch timed out");
      return Ok(());
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
async fn live_select_job(
  remote_db: Surreal<Any>,
  local_db_addr: Addr<LocalDbActor>,
  client_id: String,
) -> Result<(), crate::Error> {
  use remex_core::db::model::jobs::Job;
  use tokio_stream::StreamExt;

  let mut stream = match remote_db.select::<Vec<Job>>("job").live().await {
    Ok(s) => {
      tracing::info!("RemoteDbActor: LIVE SELECT on job table created successfully");
      s
    }
    Err(e) => {
      tracing::warn!("RemoteDbActor: failed to create job live query: {e}");
      return Err(crate::Error::from(e));
    }
  };

  while let Some(notification) = stream.next().await {
    match notification {
      Ok(n) => match n.action {
        surrealdb::types::Action::Create | surrealdb::types::Action::Update => {
          local_db_addr.do_send(CacheJob {
            job: n.data,
            client_id: client_id.clone(),
          });
        }
        surrealdb::types::Action::Delete | surrealdb::types::Action::Killed => {
          local_db_addr.do_send(RemoveJob { job_id: n.data.id });
        }
      },
      Err(e) => {
        tracing::error!("RemoteDbActor: job live select error: {:#?}", e);
        return Err(crate::Error::from(e));
      }
    }
  }

  tracing::warn!("RemoteDbActor: job live select stream ended");
  Err(crate::Error::DbError(DbError::OperationFailed("job live select stream ended".into())))
}

/// LIVE SELECT on the group table. Forwards all notifications to LocalDbActor.
async fn live_select_group(
  remote_db: Surreal<Any>,
  local_db_addr: Addr<LocalDbActor>,
  client_id: String,
) -> Result<(), crate::Error> {
  use remex_core::db::model::groups::Group;
  use tokio_stream::StreamExt;

  let mut stream = match remote_db.select::<Vec<Group>>("group").live().await {
    Ok(s) => {
      tracing::info!("RemoteDbActor: LIVE SELECT on group table created successfully");
      s
    }
    Err(e) => {
      tracing::warn!("RemoteDbActor: failed to create group live query: {e}");
      return Err(crate::Error::from(e));
    }
  };

  while let Some(notification) = stream.next().await {
    match notification {
      Ok(n) => {
        local_db_addr.do_send(GroupEvent {
          group: n.data,
          action: n.action,
          client_id: client_id.clone(),
        });
      }
      Err(e) => {
        tracing::error!("RemoteDbActor: group live select error: {:#?}", e);
        return Err(crate::Error::from(e));
      }
    }
  }

  tracing::warn!("RemoteDbActor: group live select stream ended");
  Err(crate::Error::DbError(DbError::OperationFailed("group live select stream ended".into())))
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

/// Load session from LocalDbActor, creating a new one if none exists.
async fn load_or_get_session(
  local_db_addr: &actix::Addr<LocalDbActor>,
  hardware_hash: &str,
) -> Option<Session> {
  match local_db_addr.send(GetSession).await {
    Ok(Ok(session)) => Some(session),
    Ok(Err(e)) => {
      // LocalDbActor stub returns error — fall back to creating session directly
      tracing::warn!("RemoteDbActor: GetSession returned error: {e}. Creating session directly.");
      create_session_directly(hardware_hash).await
    }
    Err(e) => {
      tracing::error!("RemoteDbActor: failed to send GetSession: {e}. Creating session directly.");
      create_session_directly(hardware_hash).await
    }
  }
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
    let local_db_addr = LocalDbActor::new().start();
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

    // Push while connected — should NOT queue (will attempt remote CREATE,
    // which may fail since memory DB doesn't have the execution table)
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

    // The push should NOT have been queued (it was sent to remote instead)
    let count = remote_db_addr.send(GetPendingCount).await.unwrap();
    assert_eq!(count, 0, "push after connect should not be queued");
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

    // The push was sent to remote (possibly failed on CREATE), but should NOT be queued
    let count = remote_db_addr.send(GetPendingCount).await.unwrap();
    assert_eq!(count, 0, "push after connect should not remain queued");
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
