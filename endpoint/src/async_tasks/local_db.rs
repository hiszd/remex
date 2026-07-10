use std::time::Duration;

use actix::prelude::*;
use remex_core::db::{
  model::executions::ExecutionStatus,
  DbError,
  DbOperator,
};
use surrealdb::{
  engine::local::Db,
  types::ToSql,
  Surreal,
};

use crate::{
  async_tasks::{
    jobs::{
      execution::mark_job_completed,
      sync::sync_job_to_cache,
    },
    remote_db::RemoteDbActor,
    CacheJob,
    GetSession,
    MarkExecutionSynced,
    PushExecution,
    RecordExecution,
    SaveSession,
    SetRemoteDbAddr,
  },
  db::{
    endpoint::Session,
    last_action::LastAction,
    remex::{
      ExecutionCache,
      ExecutionCacheData,
      JobCache,
      SurrealExecutionCacheRepo,
      SurrealJobCacheRepo,
    },
  },
};

// ── Private tick messages for periodic tasks ──

#[derive(Message)]
#[rtype(result = "()")]
struct ExecutionSyncTick;

#[derive(Message)]
#[rtype(result = "()")]
struct CleanupTick;

// ── Actor ──

pub struct LocalDbActor {
  /// Handle to the local SurrealKV connection (shared via LOCALD_B).
  local_db: Surreal<Db>,
  /// Address of the RemoteDbActor for pushing unsynced executions.
  remote_db_addr: Option<Addr<RemoteDbActor>>,
  /// In-memory session cache (loaded from local DB on startup/restart).
  session: Option<Session>,
}

impl LocalDbActor {
  pub fn new() -> Self {
    Self {
      local_db: crate::db::LOCAL_DB.clone(),
      remote_db_addr: None,
      session: None,
    }
  }
}

impl Actor for LocalDbActor {
  type Context = Context<Self>;

  fn started(&mut self, ctx: &mut Self::Context) {
    tracing::info!("LocalDbActor: started");

    // Load session from local DB asynchronously
    let db = self.local_db.clone();
    let addr = ctx.address();
    tokio::spawn(async move {
      // Set the ns/db context on the cloned handle
      if let Err(e) = db.use_ns("remex").use_db("endpoint").await {
        tracing::error!("LocalDbActor: failed to set ns/db: {e}");
        return;
      }
      match db.query("SELECT * FROM session;").await {
        Ok(mut response) => match response.take::<Vec<Session>>(0) {
          Ok(sessions) => {
            if let Some(session) = sessions.into_iter().next() {
              tracing::info!("LocalDbActor: loaded session {}", session.session_id());
              addr.do_send(SessionLoaded(session));
            } else {
              tracing::warn!("LocalDbActor: no session found in local DB");
            }
          }
          Err(e) => {
            tracing::error!("LocalDbActor: failed to deserialize session: {e}");
          }
        },
        Err(e) => {
          tracing::error!("LocalDbActor: failed to query session: {e}");
        }
      }
    });

    // Schedule periodic tasks
    ctx.notify_later(ExecutionSyncTick, Duration::from_secs(30));
    ctx.notify_later(CleanupTick, Duration::from_secs(30));
  }
}

impl actix::Supervised for LocalDbActor {
  fn restarting(&mut self, ctx: &mut Self::Context) {
    tracing::info!("LocalDbActor: restarting — reloading session");

    // Re-load session from local DB
    let db = self.local_db.clone();
    let addr = ctx.address();
    tokio::spawn(async move {
      if let Err(e) = db.use_ns("remex").use_db("endpoint").await {
        tracing::error!("LocalDbActor: failed to set ns/db on restart: {e}");
        return;
      }
      match db.query("SELECT * FROM session;").await {
        Ok(mut response) => match response.take::<Vec<Session>>(0) {
          Ok(sessions) => {
            if let Some(session) = sessions.into_iter().next() {
              tracing::info!("LocalDbActor: reloaded session {}", session.session_id());
              addr.do_send(SessionLoaded(session));
            } else {
              tracing::warn!("LocalDbActor: no session found on restart");
            }
          }
          Err(e) => {
            tracing::error!("LocalDbActor: failed to deserialize session on restart: {e}");
          }
        },
        Err(e) => {
          tracing::error!("LocalDbActor: failed to query session on restart: {e}");
        }
      }
    });

    // Re-schedule periodic tasks (started() is NOT called on restart with Supervisor)
    ctx.notify_later(ExecutionSyncTick, Duration::from_secs(30));
    ctx.notify_later(CleanupTick, Duration::from_secs(30));
  }
}

// ── Internal message: session loaded from DB ──

#[derive(Message)]
#[rtype(result = "()")]
struct SessionLoaded(Session);

impl Handler<SessionLoaded> for LocalDbActor {
  type Result = ();

  fn handle(&mut self, msg: SessionLoaded, _ctx: &mut Self::Context) {
    self.session = Some(msg.0);
    tracing::debug!("LocalDbActor: session loaded into memory");
  }
}

// ── Wire-up ──

impl Handler<SetRemoteDbAddr> for LocalDbActor {
  type Result = ();

  fn handle(&mut self, msg: SetRemoteDbAddr, _ctx: &mut Self::Context) {
    self.remote_db_addr = Some(msg.0);
    tracing::info!("LocalDbActor: remote_db_addr set");
  }
}

// ── Session handlers ──

impl Handler<GetSession> for LocalDbActor {
  type Result = Result<Session, DbError>;

  fn handle(&mut self, _msg: GetSession, _ctx: &mut Self::Context) -> Self::Result {
    match &self.session {
      Some(session) => Ok(session.clone()),
      None => Err(DbError::OperationFailed("LocalDbActor: session not loaded".into())),
    }
  }
}

impl Handler<SaveSession> for LocalDbActor {
  type Result = ();

  fn handle(&mut self, msg: SaveSession, _ctx: &mut Self::Context) {
    if let Some(ref mut session) = self.session {
      session.client_id = Some(msg.client_id.clone());
      session.secret = Some(msg.secret.clone());

      // Persist to local DB asynchronously
      let db = self.local_db.clone();
      let session_id = session.session_id();
      let client_id = msg.client_id.clone();
      let secret = msg.secret.clone();

      tokio::spawn(async move {
        let rid = surrealdb::types::RecordId::new("session", session_id.as_str());
        match db
          .query(
            "USE NS remex DB endpoint; UPDATE $id MERGE { client_id: $client_id, secret: $secret };",
          )
          .bind(("id", rid))
          .bind(("client_id", client_id))
          .bind(("secret", secret))
          .await
        {
          Ok(_) => tracing::debug!("LocalDbActor: session saved"),
          Err(e) => tracing::error!("LocalDbActor: failed to save session: {e}"),
        }
      });
    } else {
      tracing::warn!("LocalDbActor: SaveSession called but session is not loaded");
    }
  }
}

// ── Execution handlers ──

impl Handler<RecordExecution> for LocalDbActor {
  type Result = ();

  fn handle(&mut self, msg: RecordExecution, _ctx: &mut Self::Context) {
    let result = msg.result;

    // Construct an Execution from the ExecutionResult
    let execution =
      surrealdb::types::RecordId::new("execution", uuid::Uuid::new_v4().to_string().as_str());
    let execution = remex_core::db::model::executions::Execution {
      id: execution,
      job_id: Some(result.job_id.clone()),
      client_id: result.client_id.clone(),
      status: result.status.clone(),
      output: result.output.clone(),
      command: String::new(),
      exit_code: result.exit_code.clone(),
      execution_start: result.execution_start,
      execution_end: result.execution_end,
      created_at: surrealdb::types::Datetime::now(),
      updated_at: surrealdb::types::Datetime::now(),
    };

    // Spawn the DB work
    let db = self.local_db.clone();
    let job_id_str = result.job_id.to_sql();
    tokio::spawn(async move {
      if let Err(e) = db.use_ns("remex").use_db("remex").await {
        tracing::error!("LocalDbActor: failed to set ns/db for RecordExecution: {e}");
        return;
      }

      // Create cache entry
      let cache_data = ExecutionCacheData {
        execution_id: execution.id.to_sql(),
        execution_info: execution.clone(),
        synced: false,
      };
      let repo = SurrealExecutionCacheRepo { db: db.clone() };
      match repo.create(cache_data).await {
        Ok(cache) => {
          tracing::debug!("LocalDbActor: recorded execution {} in cache", cache.cache_id());
        }
        Err(e) => {
          tracing::error!("LocalDbActor: failed to create execution cache entry: {e}");
          return;
        }
      }

      // Mark job completed if execution was successful
      if execution.status == ExecutionStatus::Completed {
        let job_repo = SurrealJobCacheRepo { db: db.clone() };
        if let Err(e) = mark_job_completed(&job_id_str, &job_repo).await {
          tracing::warn!("LocalDbActor: failed to mark job {} completed: {e}", job_id_str);
        }
      }
    });
  }
}

impl Handler<MarkExecutionSynced> for LocalDbActor {
  type Result = ();

  fn handle(&mut self, msg: MarkExecutionSynced, _ctx: &mut Self::Context) {
    let db = self.local_db.clone();
    let cache_id = msg.cache_id.clone();

    tokio::spawn(async move {
      let rid = surrealdb::types::RecordId::new("execution", cache_id.as_str());
      match db
        .query("USE NS remex DB remex; UPDATE $id MERGE { synced: true };")
        .bind(("id", rid))
        .await
      {
        Ok(_) => tracing::debug!("LocalDbActor: marked execution {cache_id} as synced"),
        Err(e) => {
          tracing::error!("LocalDbActor: failed to mark execution {cache_id} as synced: {e}")
        }
      }
    });
  }
}

// ── Job caching ──

impl Handler<CacheJob> for LocalDbActor {
  type Result = ();

  fn handle(&mut self, msg: CacheJob, _ctx: &mut Self::Context) {
    let db = self.local_db.clone();
    let job = msg.job;

    tokio::spawn(async move {
      if let Err(e) = db.use_ns("remex").use_db("remex").await {
        tracing::error!("LocalDbActor: failed to set ns/db for CacheJob: {e}");
        return;
      }

      // Check if job already exists in cache
      let existing: Vec<JobCache> = match db
        .query("SELECT * FROM job WHERE job_id = $job_id LIMIT 1;")
        .bind(("job_id", job.id.to_sql()))
        .await
      {
        Ok(mut res) => match res.take(0) {
          Ok(v) => v,
          Err(e) => {
            tracing::warn!("LocalDbActor: failed to deserialize existing cache: {e}");
            vec![]
          }
        },
        Err(e) => {
          tracing::warn!("LocalDbActor: failed to query existing cache: {e}");
          vec![]
        }
      };

      let repo = SurrealJobCacheRepo { db: db.clone() };
      match sync_job_to_cache(&job, existing.first(), &repo).await {
        Ok(cached) => {
          tracing::debug!(
            "LocalDbActor: cached job {} (completed={})",
            cached.job_id,
            cached.completed
          );
        }
        Err(e) => {
          tracing::error!("LocalDbActor: failed to cache job {}: {e}", job.job_name);
        }
      }
    });
  }
}

// ── Periodic sync ──

impl Handler<ExecutionSyncTick> for LocalDbActor {
  type Result = ();

  fn handle(&mut self, _msg: ExecutionSyncTick, ctx: &mut Self::Context) {
    // Re-schedule first (self-healing: even if this tick fails, next one will run)
    ctx.notify_later(ExecutionSyncTick, Duration::from_secs(30));

    let remote_db_addr = match &self.remote_db_addr {
      Some(addr) => addr.clone(),
      None => {
        tracing::debug!("LocalDbActor: remote_db_addr not set, skipping execution sync");
        return;
      }
    };

    let db = self.local_db.clone();

    tokio::spawn(async move {
      let unsynced: Vec<ExecutionCache> = match db
        .query("USE NS remex DB remex; SELECT * FROM execution WHERE synced = false;")
        .await
      {
        Ok(mut res) => match res.take(0) {
          Ok(v) => v,
          Err(e) => {
            tracing::warn!("LocalDbActor: failed to deserialize unsynced executions: {e}");
            return;
          }
        },
        Err(e) => {
          tracing::warn!("LocalDbActor: failed to query unsynced executions: {e}");
          return;
        }
      };

      if unsynced.is_empty() {
        return;
      }

      tracing::info!("LocalDbActor: pushing {} unsynced executions to remote", unsynced.len());

      for entry in unsynced {
        remote_db_addr.do_send(PushExecution {
          cache_id: entry.cache_id(),
          execution: entry.execution_info,
        });
      }
    });
  }
}

// ── Periodic cleanup ──

impl Handler<CleanupTick> for LocalDbActor {
  type Result = ();

  fn handle(&mut self, _msg: CleanupTick, ctx: &mut Self::Context) {
    // Re-schedule first
    ctx.notify_later(CleanupTick, Duration::from_secs(30));

    let db = self.local_db.clone();

    tokio::spawn(async move {
      // Throttle: only run cleanup every 6 hours
      match LastAction::should_skip(&db, "cleanup_executions", 6 * 3600).await {
        Ok(true) => {
          tracing::debug!("LocalDbActor: cleanup throttled — skipping");
          return;
        }
        Ok(false) => { /* proceed */ }
        Err(e) => {
          tracing::warn!("LocalDbActor: failed to check cleanup throttle: {e}");
          return;
        }
      }

      match db
        .query(
          "USE NS remex DB remex; DELETE execution WHERE synced = true AND created_at < time::now() - 7d;",
        )
        .await
      {
        Ok(_) => {
          tracing::debug!("LocalDbActor: execution cleanup completed");
          // Record the last run time
          if let Err(e) = LastAction::record(&db, "cleanup_executions").await {
            tracing::warn!("LocalDbActor: failed to record cleanup time: {e}");
          }
          if let Err(e) = LastAction::cleanup_old(&db).await {
            tracing::warn!("LocalDbActor: failed to clean old last_action records: {e}");
          }
        }
        Err(e) => {
          tracing::warn!("LocalDbActor: execution cleanup failed: {e}");
        }
      }
    });
  }
}

// ── Tests ──

#[cfg(test)]
mod local_db_tests {
  use actix::prelude::*;
  use remex_core::db::model::{
    executions::ExecutionStatus,
    jobs::{
      Enabled,
      ExecutionStatus as JobExecStatus,
      Job,
      JobType,
    },
  };
  use surrealdb::{
    types::ToSql,
    Surreal,
  };

  use super::{
    ExecutionSyncTick,
    LocalDbActor,
  };
  use crate::{
    async_tasks::{
      CacheJob,
      GetSession,
      MarkExecutionSynced,
      RecordExecution,
      SaveSession,
    },
    db::endpoint::Session,
  };

  // ── Helpers ──

  /// Create a fresh temporary SurrealDB for testing with the required tables.
  /// Each test gets a unique path to avoid lock conflicts.
  async fn setup_temp_db() -> (Surreal<surrealdb::engine::local::Db>, String) {
    let db_path = format!("/tmp/remex_test_{}", uuid::Uuid::new_v4());
    let db: Surreal<surrealdb::engine::local::Db> = Surreal::init();
    // SurrealKv with a file path creates the directory automatically
    db.connect::<surrealdb::engine::local::SurrealKv>(&db_path)
      .await
      .unwrap();

    // Set up the endpoint DB with session table
    db.use_ns("remex").use_db("endpoint").await.unwrap();
    db.query(
      r#"
      DEFINE TABLE IF NOT EXISTS session SCHEMAFULL;
      DEFINE FIELD IF NOT EXISTS client_id ON TABLE session TYPE option<string>;
      DEFINE FIELD IF NOT EXISTS client_name ON TABLE session TYPE string;
      DEFINE FIELD IF NOT EXISTS hardware_hash ON TABLE session TYPE string;
      DEFINE FIELD IF NOT EXISTS db_addr ON TABLE session TYPE option<string>;
      DEFINE FIELD IF NOT EXISTS tkn ON TABLE session TYPE option<object> FLEXIBLE;
      DEFINE FIELD IF NOT EXISTS secret ON TABLE session TYPE option<string>;
      DEFINE FIELD IF NOT EXISTS groups ON TABLE session TYPE array<record<group>>;
      "#,
    )
    .await
    .unwrap()
    .check()
    .unwrap();

    // Create a test session
    db.query(
      r#"
      CREATE session CONTENT {
        client_id: NONE,
        client_name: "test-client",
        hardware_hash: "test-hash",
        db_addr: NONE,
        tkn: NONE,
        secret: NONE,
        groups: []
      };
      "#,
    )
    .await
    .unwrap()
    .check()
    .unwrap();

    // Set up the remex DB with job and execution cache tables
    db.use_ns("remex").use_db("remex").await.unwrap();
    db.query(
      r#"
      DEFINE TABLE IF NOT EXISTS job SCHEMAFULL;
      DEFINE FIELD IF NOT EXISTS job_id ON TABLE job TYPE string;
      DEFINE FIELD IF NOT EXISTS job_info ON TABLE job TYPE object FLEXIBLE;
      DEFINE FIELD IF NOT EXISTS completed ON TABLE job TYPE bool DEFAULT false;
      "#,
    )
    .await
    .unwrap()
    .check()
    .unwrap();

    db.query(
      r#"
      DEFINE TABLE IF NOT EXISTS execution SCHEMAFULL;
      DEFINE FIELD IF NOT EXISTS execution_id ON TABLE execution TYPE string;
      DEFINE FIELD IF NOT EXISTS execution_info ON TABLE execution TYPE object FLEXIBLE;
      DEFINE FIELD IF NOT EXISTS synced ON TABLE execution TYPE bool DEFAULT false;
      "#,
    )
    .await
    .unwrap()
    .check()
    .unwrap();

    // Set up the endpoint DB for last_action table
    db.use_ns("remex").use_db("endpoint").await.unwrap();
    db.query(
      r#"
      DEFINE TABLE IF NOT EXISTS last_action SCHEMAFULL;
      DEFINE FIELD IF NOT EXISTS task_name ON TABLE last_action TYPE string;
      DEFINE FIELD IF NOT EXISTS last_run ON TABLE last_action TYPE datetime;
      "#,
    )
    .await
    .unwrap()
    .check()
    .unwrap();

    // Switch back to endpoint for session queries
    db.use_ns("remex").use_db("endpoint").await.unwrap();
    (db, db_path)
  }

  /// Create a fresh LocalDbActor connected to a temp database.
  /// Returns (actor_addr, db_handle, _db_path_guard).
  async fn setup_actor() -> (Addr<LocalDbActor>, Surreal<surrealdb::engine::local::Db>, String) {
    let (db, db_path) = setup_temp_db().await;

    let actor = LocalDbActor {
      local_db: db.clone(),
      remote_db_addr: None,
      session: None,
    };

    let addr = actor.start();

    // Poll until session is loaded (with timeout)
    let started = std::time::Instant::now();
    let timeout = std::time::Duration::from_secs(5);
    loop {
      match addr.send(GetSession).await {
        Ok(Ok(session)) => {
          assert_eq!(session.client_name, "test-client");
          break;
        }
        _ => {
          if started.elapsed() > timeout {
            panic!("timed out waiting for session to load");
          }
          tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }
      }
    }

    (addr, db, db_path)
  }

  fn make_test_job(id: &str, name: &str) -> Job {
    Job {
      id: surrealdb::types::RecordId::new("job", id),
      job_name: name.to_string(),
      job_shell: "/bin/sh".to_string(),
      job_command: "echo hello".to_string(),
      job_type: JobType::Instant,
      execution_status: JobExecStatus::Pending,
      enabled: Enabled::Enabled,
      assignments: vec![],
      timeout: None,
      created_at: surrealdb::types::Datetime::default(),
      updated_at: surrealdb::types::Datetime::default(),
    }
  }

  /// Helper to get the actor's internal session (read-only snapshot).
  /// We do this by sending GetSession and checking the result.
  async fn get_session(addr: &Addr<LocalDbActor>) -> Result<Session, remex_core::db::DbError> {
    addr.send(GetSession).await.unwrap()
  }

  // ── Tests ──

  #[actix::test]
  async fn get_session_none_when_not_loaded() {
    // Create a DB without a session record so started() won't load one
    let db_path = format!("/tmp/remex_test_{}", uuid::Uuid::new_v4());
    let db: Surreal<surrealdb::engine::local::Db> = Surreal::init();
    db.connect::<surrealdb::engine::local::SurrealKv>(&db_path)
      .await
      .unwrap();
    // Create the table but don't insert a session
    db.use_ns("remex").use_db("endpoint").await.unwrap();
    db.query(
      r#"
      DEFINE TABLE IF NOT EXISTS session SCHEMAFULL;
      DEFINE FIELD IF NOT EXISTS client_id ON TABLE session TYPE option<string>;
      DEFINE FIELD IF NOT EXISTS client_name ON TABLE session TYPE string;
      DEFINE FIELD IF NOT EXISTS hardware_hash ON TABLE session TYPE string;
      "#,
    )
    .await
    .unwrap()
    .check()
    .unwrap();

    let actor = LocalDbActor {
      local_db: db.clone(),
      remote_db_addr: None,
      session: None,
    };
    let addr = actor.start();
    // Wait long enough for started() to query and find no session
    tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

    let result = get_session(&addr).await;
    match result {
      Ok(_) => panic!("expected error when session is not loaded"),
      Err(e) => {
        let msg = format!("{e}");
        assert!(
          msg.contains("session not loaded"),
          "expected 'session not loaded' error, got: {msg}"
        );
      }
    }
  }

  #[actix::test]
  async fn get_session_returns_loaded_session() {
    let (addr, _db, _dir) = setup_actor().await;

    let result = get_session(&addr).await;
    match result {
      Ok(session) => {
        assert_eq!(session.client_name, "test-client");
        assert_eq!(session.hardware_hash, "test-hash");
        assert_eq!(session.client_id, None);
      }
      Err(e) => panic!("expected session to be loaded: {e}"),
    }
  }

  #[actix::test]
  async fn save_session_persists_and_updates_state() {
    let (addr, _db, _dir) = setup_actor().await;

    // Send SaveSession
    addr
      .send(SaveSession {
        client_id: "client:test-123".to_string(),
        secret: "test-secret".to_string(),
      })
      .await
      .unwrap();

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Verify in-memory state via GetSession
    let session = get_session(&addr).await.unwrap();
    assert_eq!(
      session.client_id,
      Some("client:test-123".to_string()),
      "session should have updated client_id"
    );
    assert_eq!(
      session.secret,
      Some("test-secret".to_string()),
      "session should have updated secret"
    );
  }

  #[actix::test]
  async fn save_session_overwrites_previous() {
    let (addr, _db, _dir) = setup_actor().await;

    // First save
    addr
      .send(SaveSession {
        client_id: "client:first".to_string(),
        secret: "secret-first".to_string(),
      })
      .await
      .unwrap();

    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    // Second save (overwrite)
    addr
      .send(SaveSession {
        client_id: "client:second".to_string(),
        secret: "secret-second".to_string(),
      })
      .await
      .unwrap();

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let session = get_session(&addr).await.unwrap();
    assert_eq!(
      session.client_id,
      Some("client:second".to_string()),
      "session should have the latest client_id"
    );
    assert_eq!(
      session.secret,
      Some("secret-second".to_string()),
      "session should have the latest secret"
    );
  }

  #[actix::test]
  async fn mark_synced_updates_cache_entry() {
    let (db, _dir) = setup_temp_db().await;
    let remex_db = db.clone();

    // Create an unsynced execution cache entry directly in the remex DB
    remex_db.use_ns("remex").use_db("remex").await.unwrap();
    let cache_id = "mark-synced-test";
    let rid = surrealdb::types::RecordId::new("execution", cache_id);
    remex_db
      .query(
        "USE NS remex DB remex; CREATE $id CONTENT {
          execution_id: 'execution:test',
          execution_info: {},
          synced: false
        };",
      )
      .bind(("id", rid.clone()))
      .await
      .unwrap()
      .check()
      .unwrap();
    // Switch back to endpoint for actor startup
    remex_db.use_ns("remex").use_db("endpoint").await.unwrap();

    let actor = LocalDbActor {
      local_db: db.clone(),
      remote_db_addr: None,
      session: None,
    };
    let addr = actor.start();
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    // Send MarkExecutionSynced
    addr
      .send(MarkExecutionSynced {
        cache_id: cache_id.to_string(),
      })
      .await
      .unwrap();

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Verify the entry was updated
    remex_db.use_ns("remex").use_db("remex").await.unwrap();
    let result: Vec<serde_json::Value> = remex_db
      .query("SELECT * FROM execution WHERE id = $id")
      .bind(("id", rid))
      .await
      .unwrap()
      .check()
      .unwrap()
      .take(0)
      .unwrap();

    assert_eq!(result.len(), 1, "entry should exist");
    assert_eq!(result[0]["synced"], true, "entry should be marked synced");
  }

  #[actix::test]
  async fn cache_job_creates_cache_entry() {
    let (addr, db, _dir) = setup_actor().await;
    let job = make_test_job("cache-test-job", "cached-job");

    // Send CacheJob
    addr.send(CacheJob { job: job.clone() }).await.unwrap();

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Verify the entry was created in the local cache
    db.use_ns("remex").use_db("remex").await.unwrap();
    let result: Vec<serde_json::Value> = db
      .query("SELECT * FROM job WHERE job_id = $job_id")
      .bind(("job_id", job.id.to_sql()))
      .await
      .unwrap()
      .check()
      .unwrap()
      .take(0)
      .unwrap();

    assert!(!result.is_empty(), "cache entry should exist");
    assert_eq!(result[0]["job_info"]["job_name"], "cached-job", "cache should contain the job");
  }

  #[actix::test]
  async fn execution_sync_tick_no_crash_when_no_remote_addr() {
    // Send ExecutionSyncTick without setting remote_db_addr — should log and skip
    let (addr, _db, _dir) = setup_actor().await;
    addr.send(ExecutionSyncTick).await.unwrap();
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
    // No panic = success
  }

  #[actix::test]
  async fn record_execution_creates_cache_entry() {
    use crate::async_tasks::jobs::execution::ExecutionResult;

    let (addr, db, _dir) = setup_actor().await;

    let result = ExecutionResult {
      output: "test output".to_string(),
      exit_code: "0".to_string(),
      execution_start: surrealdb::types::Datetime::now(),
      execution_end: Some(surrealdb::types::Datetime::now()),
      job_id: surrealdb::types::RecordId::new("job", "record-test-job"),
      client_id: surrealdb::types::RecordId::new("client", "test-client"),
      status: ExecutionStatus::Completed,
    };

    addr.send(RecordExecution { result }).await.unwrap();

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Verify a cache entry was created
    db.use_ns("remex").use_db("remex").await.unwrap();
    let execs: Vec<serde_json::Value> = db
      .query("SELECT * FROM execution;")
      .await
      .unwrap()
      .check()
      .unwrap()
      .take(0)
      .unwrap();

    assert!(!execs.is_empty(), "execution cache entry should exist");
    // Each execution in the cache has a synced field
    for exec in &execs {
      assert_eq!(exec["synced"], false, "new execution should be unsynced");
    }
  }
}
