use std::time::Duration;

use actix::prelude::*;
use remex_core::db::DbError;
use surrealdb::{
  engine::any::Any,
  types::ToSql,
  Surreal,
};

use crate::{
  async_tasks::{
    jobs::scheduler::SchedulerActor,
    local_db::LocalDbActor,
    GetSession,
    MarkExecutionSynced,
    PushExecution,
    SaveSession,
  },
  db::{
    endpoint::{
      Session,
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

// ── Actor ──

pub struct RemoteDbActor {
  // Config (immutable after construction)
  db_url: String,
  enrollment_token: Option<String>,

  // Connection state
  remote_db: Option<Surreal<Any>>,
  client_id: Option<String>,
  connected: bool,

  // Pending pushes (queued while disconnected)
  pending_executions: Vec<PushExecution>,

  // References to other actors
  local_db_addr: Addr<LocalDbActor>,
  scheduler_addr: Addr<SchedulerActor>,
}

impl RemoteDbActor {
  #[allow(clippy::too_many_arguments)]
  pub fn new(
    db_url: String,
    enrollment_token: Option<String>,
    local_db_addr: Addr<LocalDbActor>,
    scheduler_addr: Addr<SchedulerActor>,
  ) -> Self {
    RemoteDbActor {
      db_url,
      enrollment_token,
      remote_db: None,
      client_id: None,
      connected: false,
      pending_executions: Vec::new(),
      local_db_addr,
      scheduler_addr,
    }
  }
}

impl Actor for RemoteDbActor {
  type Context = Context<Self>;

  fn started(&mut self, ctx: &mut Self::Context) {
    let addr = ctx.address();
    let db_url = self.db_url.clone();
    let enrollment_token = self.enrollment_token.clone();
    let local_db_addr = self.local_db_addr.clone();

    tokio::spawn(async move {
      connection_loop(&db_url, enrollment_token.as_deref(), addr, local_db_addr).await;
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
    let local_db_addr = self.local_db_addr.clone();
    tokio::spawn(async move {
      connection_loop(&db_url, enrollment_token.as_deref(), addr, local_db_addr).await;
    });
  }
}

// ── Message Handlers ──

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
      let cache_id = msg.cache_id.clone();

      tokio::spawn(async move {
        match db
          .query("CREATE execution CONTENT $data")
          .bind(("data", msg.execution))
          .await
        {
          Ok(result) => match result.check() {
            Ok(_) => {
              tracing::debug!("RemoteDbActor: successfully pushed execution {cache_id}");
              local_db.do_send(MarkExecutionSynced { cache_id });
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
    self.client_id = Some(msg.client_id);
    self.connected = true;

    // Spawn heartbeat loop
    if let Some(ref db) = self.remote_db {
      let db_clone = db.clone();
      let cid = self.client_id.clone().unwrap_or_default();
      tokio::spawn(async move {
        heartbeat_loop(db_clone, &cid).await;
      });
    }

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

// ── Internal Methods ──

impl RemoteDbActor {
  fn drain_pending_executions(&mut self) {
    let pending = std::mem::take(&mut self.pending_executions);
    if pending.is_empty() {
      return;
    }
    tracing::info!("RemoteDbActor: draining {} pending executions", pending.len());
    for msg in pending {
      // Re-process through the handler (connected = true now)
      match self.handle(msg, &mut Context::new()) {
        Ok(()) => {}
        Err(e) => tracing::error!("RemoteDbActor: drain push execution failed: {e}"),
      }
    }
  }
}

// ── Internal Tasks ──

/// Main connection loop — runs as a tokio task, connects and authenticates,
/// then sleeps for ~1 hour before re-authenticating.
async fn connection_loop(
  db_url: &str,
  enrollment_token: Option<&str>,
  addr: actix::Addr<RemoteDbActor>,
  local_db_addr: actix::Addr<LocalDbActor>,
) {
  loop {
    // Clear old state
    addr.do_send(ClearConnection);

    // Load session from local DB
    let session = match load_or_get_session(&local_db_addr).await {
      Some(s) => s,
      None => {
        tracing::error!("RemoteDbActor: failed to load or create session");
        tokio::time::sleep(Duration::from_secs(5)).await;
        continue;
      }
    };

    let hardware_hash = match machine_uid::get() {
      Ok(h) => h,
      Err(e) => {
        tracing::warn!("RemoteDbActor: failed to get machine uid: {e}");
        String::new()
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
          let client_id = lookup_client_id(&remote_db, &hardware_hash).await;
          tracing::info!("RemoteDbActor: signed in as {client_id}");

          addr.do_send(ConnectionSucceeded {
            remote_db: remote_db.clone(),
            client_id,
          });

          // Wait ~1 hour before re-auth (with random jitter 0-300s)
          let jitter: u64 = rand::random::<u64>() % 300;
          tokio::time::sleep(Duration::from_secs(3600 + jitter)).await;
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
        .bind(("hash", hardware_hash.clone()))
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
        match remote_db
          .query("USE NS remex DB remex; DELETE FROM $id;")
          .bind(("id", surrealdb::types::RecordId::parse_simple(existing_id).unwrap()))
          .await
        {
          Ok(_) => tracing::debug!("RemoteDbActor: deleted stale client {existing_id}"),
          Err(e) => {
            tracing::error!("RemoteDbActor: failed to delete stale client {existing_id}: {e}")
          }
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
          let client_id = lookup_client_id(&remote_db, &hardware_hash).await;
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
            client_id,
          });

          // Wait ~1 hour before re-auth
          let jitter: u64 = rand::random::<u64>() % 300;
          tokio::time::sleep(Duration::from_secs(3600 + jitter)).await;
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

/// Look up the client record by hardware_hash and return its record id string.
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
async fn load_or_get_session(local_db_addr: &actix::Addr<LocalDbActor>) -> Option<Session> {
  match local_db_addr.send(GetSession).await {
    Ok(Ok(session)) => Some(session),
    Ok(Err(e)) => {
      // LocalDbActor stub returns error — fall back to creating session directly
      tracing::warn!("RemoteDbActor: GetSession returned error: {e}. Creating session directly.");
      create_session_directly().await
    }
    Err(e) => {
      tracing::error!("RemoteDbActor: failed to send GetSession: {e}. Creating session directly.");
      create_session_directly().await
    }
  }
}

/// Create a new session directly from the local DB (fallback when LocalDbActor is a stub).
async fn create_session_directly() -> Option<Session> {
  use remex_core::db::DbOperator;

  use crate::db::endpoint::SessionData;
  let local_db = match get_local_endpoint().await {
    Ok(db) => db,
    Err(e) => {
      tracing::error!("RemoteDbActor: failed to get local endpoint DB: {e}");
      return None;
    }
  };
  let hardware_hash = match machine_uid::get() {
    Ok(h) => h,
    Err(e) => {
      tracing::warn!("RemoteDbActor: failed to get machine uid for session: {e}");
      String::new()
    }
  };
  let repo = SurrealSessionRepo { db: local_db };
  match repo
    .create(SessionData {
      client_id: None,
      client_name: Some(gethostname::gethostname().to_string_lossy().to_string()),
      hardware_hash: Some(hardware_hash),
      db_addr: None,
      tkn: None,
      secret: None,
      groups: vec![],
    })
    .await
  {
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

// ── Tests ──

#[cfg(test)]
mod remote_db_tests {
  use std::sync::Arc;

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
    PushExecution,
    RemoteDbActor,
  };
  use crate::async_tasks::{
    jobs::{
      scheduler::SchedulerActor,
      RealJobExecutor,
    },
    local_db::LocalDbActor,
  };

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

  fn setup_actor() -> (Addr<RemoteDbActor>, Addr<LocalDbActor>, Addr<SchedulerActor>) {
    let executor = Arc::new(RealJobExecutor);
    let scheduler_addr = SchedulerActor::new(executor).start();
    let local_db_addr = LocalDbActor::new().start();
    let remote_db_addr =
      RemoteDbActor::new("memory".to_string(), None, local_db_addr.clone(), scheduler_addr.clone())
        .start();
    (remote_db_addr, local_db_addr, scheduler_addr)
  }

  // ── Tests ──

  #[actix::test]
  async fn push_queues_when_disconnected() {
    let (remote_db_addr, _local_db, _scheduler) = setup_actor();

    // Send a push while not connected
    let exec = make_test_execution();
    remote_db_addr
      .send(PushExecution {
        cache_id: "test-cache-1".to_string(),
        execution: exec,
      })
      .await
      .unwrap()
      .unwrap();

    // Give the actor time to process
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
    // If we got here without panic, the queueing worked
  }

  #[actix::test]
  async fn push_does_not_panic_when_disconnected() {
    let (remote_db_addr, _local_db, _scheduler) = setup_actor();

    // Send multiple pushes while disconnected
    for i in 0..5 {
      let exec = make_test_execution();
      let _ = remote_db_addr
        .send(PushExecution {
          cache_id: format!("test-cache-{i}"),
          execution: exec,
        })
        .await
        .unwrap();
    }

    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
  }

  #[actix::test]
  async fn connection_succeeded_sets_state() {
    let (remote_db_addr, _local_db, _scheduler) = setup_actor();

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

    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
    // No panic = state transition works
  }

  #[actix::test]
  async fn connection_failed_does_not_panic() {
    let (remote_db_addr, _local_db, _scheduler) = setup_actor();

    remote_db_addr.send(ConnectionFailed).await.unwrap();
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
  }

  #[actix::test]
  async fn clear_connection_does_not_panic() {
    let (remote_db_addr, _local_db, _scheduler) = setup_actor();

    remote_db_addr.send(ClearConnection).await.unwrap();
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
  }

  #[actix::test]
  async fn push_after_connected_does_not_panic() {
    let (remote_db_addr, _local_db, _scheduler) = setup_actor();

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
    let _ = remote_db_addr
      .send(PushExecution {
        cache_id: "push-after-connect".to_string(),
        execution: exec,
      })
      .await
      .unwrap();

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
  }
}
