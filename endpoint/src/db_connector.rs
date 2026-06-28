use remex_core::db::DbOperator;
use surrealdb::engine::any::Any;
use surrealdb::types::ToSql;
use surrealdb::Surreal;
use tokio::sync::{mpsc, watch};

use crate::async_tasks::jobs::monitor::MonitorCommand;
use crate::db::endpoint::SurrealSessionRepo;

pub async fn run(
  db_url: String,
  enrollment_token: Option<String>,
  db_handle_tx: watch::Sender<Option<(Surreal<Any>, String)>>,
  monitor_cmd_tx: mpsc::Sender<MonitorCommand>,
  heartbeat_client_id_tx: mpsc::Sender<String>,
) {
  loop {
    let local_db = match crate::db::get_local_endpoint().await {
      Ok(db) => db,
      Err(e) => {
        tracing::error!("Failed to get local endpoint: {e}");
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
        continue;
      }
    };
    let session_repo = SurrealSessionRepo { db: local_db.clone() };
    let session = load_or_create_session(&session_repo).await;

    let remote_db: Surreal<Any> = Surreal::init();

    tracing::info!("Connecting to remote database at {db_url}");
    let connect_result = tokio::time::timeout(
      tokio::time::Duration::from_secs(15),
      remote_db.connect(db_url.clone()),
    )
    .await;
    match connect_result {
      Ok(Ok(())) => tracing::info!("Connected to remote database, proceeding with auth"),
      Ok(Err(e)) => {
        tracing::error!("Failed to connect to remote database: {e}");
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
        continue;
      }
      Err(_) => {
        tracing::error!("Timed out connecting to remote database at {db_url}");
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
        continue;
      }
    }
    tracing::info!("Connected to remote database, proceeding with auth");

    let hardware_hash = machine_uid::get().unwrap_or_default();

    let has_stored_creds = session.secret.is_some() && session.client_id.is_some();

    if has_stored_creds {
      let secret = session.secret.clone().unwrap_or_default();

      tracing::info!("Signing in with existing credentials");
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
        Ok(tok) => {
          let auth_token = tok.access.as_insecure_token().to_string();
          let _ = remote_db.use_ns("remex").use_db("remex").await;
          let client_id = lookup_client_id(&remote_db, &hardware_hash).await;
          tracing::info!("Signed in successfully as {client_id}");
          send_identity(&monitor_cmd_tx, &heartbeat_client_id_tx, &client_id).await;
          tracing::info!("Connected to remote database");
          let _ = db_handle_tx.send(Some((remote_db, auth_token)));
          loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(3600)).await;
          }
        }
        Err(e) => {
          tracing::error!("Signin failed: {e}. Will attempt enrollment if token is available.");
        }
      }
    }

    if let Some(token) = enrollment_token.as_ref() {
      let client_name = gethostname::gethostname().to_string_lossy().to_string();
      let secret = remex_core::utils::generate_secret(true);

      tracing::info!("Signing up with enrollment token");
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
        Ok(tok) => {
          let auth_token = tok.access.as_insecure_token().to_string();
          let _ = remote_db.use_ns("remex").use_db("remex").await;
          let client_id = lookup_client_id(&remote_db, &hardware_hash).await;
          tracing::info!("Signed up successfully as {client_id}");
          update_session(&session_repo, &session.session_id(), client_id.clone(), Some(secret)).await;
          send_identity(&monitor_cmd_tx, &heartbeat_client_id_tx, &client_id).await;
          tracing::info!("Connected to remote database");
          let _ = db_handle_tx.send(Some((remote_db, auth_token)));
          loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(3600)).await;
          }
        }
        Err(e) => {
          tracing::error!("Signup failed (debug): {e:?}");
          tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
          continue;
        }
      }
    }

    tracing::warn!("No stored credentials and no enrollment token. Retrying in 10 seconds.");
    tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
  }
}

async fn lookup_client_id(remote_db: &Surreal<Any>, hardware_hash: &str) -> String {
  let hash = hardware_hash.to_owned();
  match remote_db
    .query("SELECT VALUE id FROM client WHERE hardware_hash = $hash;")
    .bind(("hash", hash))
    .await
  {
    Ok(mut res) => match res.take::<Vec<surrealdb::types::RecordId>>(0) {
      Ok(ids) => ids.first().map(|id| id.to_sql()).unwrap_or_default(),
      Err(_) => String::new(),
    },
    Err(_) => String::new(),
  }
}

async fn send_identity(
  monitor_cmd_tx: &mpsc::Sender<MonitorCommand>,
  heartbeat_client_id_tx: &mpsc::Sender<String>,
  client_id: &str,
) {
  let _ = monitor_cmd_tx.send(MonitorCommand::SetClientId(client_id.to_string())).await;
  let _ = heartbeat_client_id_tx.send(client_id.to_string()).await;
}

async fn update_session(
  repo: &SurrealSessionRepo,
  session_id: &str,
  client_id: String,
  secret: Option<String>,
) {
  let local_db = repo.db.clone();
  let _ = local_db
    .query(
      "USE NS remex DB endpoint;
       UPDATE $id MERGE {
         client_id: $client_id,
         secret: $secret
       };",
    )
    .bind(("id", surrealdb::types::RecordId::new("session", session_id)))
    .bind(("client_id", client_id))
    .bind(("secret", secret))
    .await;
}

async fn load_or_create_session(
  repo: &SurrealSessionRepo,
) -> crate::db::endpoint::Session {
  match repo.list().await {
    Ok(mut sessions) => {
      if let Some(session) = sessions.pop() {
        session
      } else {
        create_new_session_with_repo(repo).await
      }
    }
    Err(_) => create_new_session_with_repo(repo).await,
  }
}

pub(crate) async fn create_new_session_with_repo(
  repo: &dyn remex_core::db::DbOperator<
    Record = crate::db::endpoint::Session,
    Input = crate::db::endpoint::SessionData,
  >,
) -> crate::db::endpoint::Session {
  repo
    .create(crate::db::endpoint::SessionData {
      client_id: None,
      hardware_hash: Some(machine_uid::get().unwrap()),
      client_name: Some(gethostname::gethostname().to_string_lossy().to_string()),
      db_addr: None,
      tkn: None,
      secret: None,
      groups: vec![],
    })
    .await
    .unwrap()
}

#[cfg(test)]
mod tests {
  use remex_core::impl_in_memory_db_operator;

  use crate::db::endpoint::{Session, SessionData};

  impl_in_memory_db_operator!(InMemorySessionRepo, Session, SessionData, "session");

  #[tokio::test]
  async fn create_new_session_sets_defaults() {
    let repo = InMemorySessionRepo::new();
    let session = super::create_new_session_with_repo(&repo).await;

    assert!(!session.session_id().is_empty(), "session should have an id");
    assert!(!session.client_name.is_empty(), "client_name should be non-empty");
    assert!(!session.hardware_hash.is_empty(), "hardware_hash should be non-empty");
    assert_eq!(session.client_id, None, "new session should have no client_id");
    assert_eq!(session.secret, None, "new session should have no secret");
    assert!(session.tkn.is_none(), "new session should have no token");
  }

  #[tokio::test]
  async fn create_new_session_generates_unique_ids() {
    let repo = InMemorySessionRepo::new();
    let s1 = super::create_new_session_with_repo(&repo).await;
    let s2 = super::create_new_session_with_repo(&repo).await;

    assert_ne!(s1.session_id(), s2.session_id(), "each session must have a unique id");
  }
}
