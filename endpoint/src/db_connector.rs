use std::time::Duration;

use actix::prelude::*;
use remex_core::db::DbOperator;
use surrealdb::engine::any::Any;
use surrealdb::engine::remote::http::Http;
use surrealdb::types::ToSql;
use surrealdb::Surreal;

use crate::async_tasks::ConnectionReady;
use crate::db::endpoint::SurrealSessionRepo;

#[derive(Message)]
#[rtype(result = "()")]
pub struct Subscribe(pub Recipient<ConnectionReady>);

#[derive(Message)]
#[rtype(result = "()")]
struct ConnectionEstablished {
    db: Surreal<Any>,
    client_id: String,
}

pub struct DbConnectorActor {
    db_url: String,
    enrollment_token: Option<String>,
    subscribers: Vec<Recipient<ConnectionReady>>,
}

impl DbConnectorActor {
    pub fn new(db_url: String, enrollment_token: Option<String>) -> Self {
        DbConnectorActor {
            db_url,
            enrollment_token,
            subscribers: Vec::new(),
        }
    }
}

impl Actor for DbConnectorActor {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        let db_url = self.db_url.clone();
        let enrollment_token = self.enrollment_token.clone();
        let addr = ctx.address();

        tokio::spawn(async move {
            connection_loop(&db_url, enrollment_token.as_deref(), addr).await;
        });
    }
}

impl actix::Supervised for DbConnectorActor {
    fn restarting(&mut self, ctx: &mut Context<Self>) {
        tracing::info!("DbConnectorActor: restarting");
        // Subscribers Vec is preserved (same Actor instance)
        // Re-spawn the connection loop (like started() does)
        let db_url = self.db_url.clone();
        let enrollment_token = self.enrollment_token.clone();
        let addr = ctx.address();
        tokio::spawn(async move {
            connection_loop(&db_url, enrollment_token.as_deref(), addr).await;
        });
    }
}

impl Handler<Subscribe> for DbConnectorActor {
    type Result = ();

    fn handle(&mut self, msg: Subscribe, _ctx: &mut Self::Context) {
        self.subscribers.push(msg.0);
        tracing::debug!("DbConnectorActor: subscriber added (total {})", self.subscribers.len());
    }
}

impl Handler<ConnectionEstablished> for DbConnectorActor {
    type Result = ();

    fn handle(&mut self, msg: ConnectionEstablished, _ctx: &mut Self::Context) {
        let ready = ConnectionReady {
            db: Some(msg.db),
            client_id: Some(msg.client_id),
        };

        // Broadcast to all subscribers
        for sub in &self.subscribers {
            let sub = sub.clone();
            let ready = ready.clone();
            tokio::spawn(async move {
                if let Err(e) = sub.send(ready).await {
                    tracing::warn!("DbConnectorActor: failed to send ConnectionReady to subscriber: {e}");
                }
            });
        }
    }
}

async fn connection_loop(
    db_url: &str,
    enrollment_token: Option<&str>,
    addr: actix::Addr<DbConnectorActor>,
) {
    // Outer retry loop
    loop {
        let local_db = match crate::db::get_local_endpoint().await {
            Ok(db) => db,
            Err(e) => {
                tracing::error!("Failed to get local endpoint: {e}");
                tokio::time::sleep(Duration::from_secs(5)).await;
                continue;
            }
        };
        let session_repo = SurrealSessionRepo { db: local_db.clone() };
        let session = load_or_create_session(&session_repo).await;

        let remote_db: Surreal<Http> = Surreal::init();

        tracing::info!("Connecting to remote database at {db_url}");
        // Convert WebSocket URL to HTTP URL for HTTP client
        let http_url = db_url.replace("ws://", "http://").replace("wss://", "https://");
        let connect_result = tokio::time::timeout(
            Duration::from_secs(15),
            remote_db.connect(http_url.to_string()),
        )
        .await;
        match connect_result {
            Ok(Ok(())) => tracing::info!("Connected to remote database, proceeding with auth"),
            Ok(Err(e)) => {
                tracing::error!("Failed to connect to remote database: {e}");
                tokio::time::sleep(Duration::from_secs(5)).await;
                continue;
            }
            Err(_) => {
                tracing::error!("Timed out connecting to remote database at {db_url}");
                tokio::time::sleep(Duration::from_secs(5)).await;
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
                        "variables": {
                            "hardware_hash": hardware_hash,
                            "secret": secret,
                        },
                    }),
                })
                .await
            {
                Ok(tok) => {
                    let _tok = tok;
                    if let Err(e) = remote_db.use_ns("remex").use_db("remex").await {
                        tracing::warn!("Failed to set namespace/database after signin: {e}");
                    }
                    let client_id = lookup_client_id(&remote_db, &hardware_hash).await;
                    tracing::info!("Signed in successfully as {client_id}");

                    // Broadcast connection to the actor
                    if let Err(e) = addr.send(ConnectionEstablished {
                        db: remote_db.clone(),
                        client_id: client_id.clone(),
                    })
                    .await
                    {
                        tracing::warn!("DbConnector: failed to send ConnectionEstablished after signin: {e}");
                    }

                    // Keep the connection alive by sleeping
                    // (SurrealDB client manages reconnection internally)
                    tokio::time::sleep(Duration::from_secs(3600)).await;
                }
                Err(e) => {
                    tracing::error!("Signin failed: {e}. Will attempt enrollment if token is available.");
                    // Fall through to enrollment attempt below
                }
            }
        }

        if let Some(token) = enrollment_token {
            let client_name = gethostname::gethostname().to_string_lossy().to_string();
            let secret = remex_core::utils::generate_secret(true);

            tracing::info!("Signing up with enrollment token");
            match remote_db
                .signup(surrealdb::opt::auth::Record {
                    namespace: "remex".into(),
                    database: "remex".into(),
                    access: "endpoint_access".into(),
                    params: serde_json::json!({
                        "variables": {
                            "enrollment_token": token,
                            "client_name": client_name,
                            "secret": secret,
                            "hardware_hash": hardware_hash,
                        },
                    }),
                })
                .await
            {
                Ok(tok) => {
                    let _tok = tok;
                    if let Err(e) = remote_db.use_ns("remex").use_db("remex").await {
                        tracing::warn!("Failed to set namespace/database after signup: {e}");
                    }
                    let client_id = lookup_client_id(&remote_db, &hardware_hash).await;
                    tracing::info!("Signed up successfully as {client_id}");
                    if let Err(e) = update_session(&session_repo, &session.session_id(), client_id.clone(), Some(secret)).await {
                        tracing::error!("Failed to persist session credentials after signup: {e}. Endpoint will need re-enrollment on restart.");
                    }

                    if let Err(e) = addr.send(ConnectionEstablished {
                        db: remote_db.clone(),
                        client_id: client_id.clone(),
                    })
                    .await
                    {
                        tracing::warn!("DbConnector: failed to send ConnectionEstablished after signup: {e}");
                    }

                    tokio::time::sleep(Duration::from_secs(3600)).await;
                }
                Err(e) => {
                    tracing::error!("Signup failed (debug): {e:?}");
                    tokio::time::sleep(Duration::from_secs(10)).await;
                    continue;
                }
            }
        }

        tracing::warn!("No stored credentials and no enrollment token. Retrying in 10 seconds.");
        tokio::time::sleep(Duration::from_secs(10)).await;
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

async fn update_session(
  repo: &SurrealSessionRepo,
  session_id: &str,
  client_id: String,
  secret: Option<String>,
) -> Result<(), surrealdb::Error> {
  let local_db = repo.db.clone();
  local_db
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
    .await?;
  Ok(())
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
