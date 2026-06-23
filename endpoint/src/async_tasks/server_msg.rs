use futures::{SinkExt, StreamExt};
use remex_core::codec::{self, ClientRequest, DisconnectReason, ServerResponse};
use remex_core::db::{BearerGrantResponse, DbOperator};
use surrealdb::engine::local::Db;
use surrealdb::types::ToSql;
use surrealdb::Surreal;

use crate::async_tasks::jobs::monitor::MonitorCommand;
use crate::utils;

struct MsgState {
  client_id: Option<surrealdb::types::RecordId>,
  client_name: String,
  hardware_hash: String,
  secret: Option<String>,
  authenticated: bool,
}

pub async fn server_msg_loop(
  args_secret: Option<String>,
  args_server: String,
  args_port: String,
  db_token_tx: tokio::sync::mpsc::Sender<(BearerGrantResponse, String)>,
  monitor_cmd_tx: tokio::sync::mpsc::Sender<MonitorCommand>,
) -> Result<(), remex_core::db::DbError> {
  let local_endpoint = crate::db::get_local_endpoint().await?;

  let session = load_or_create_session(&local_endpoint).await;
  let mut state = MsgState {
    client_id: session
      .client_id
      .clone()
      .and_then(|s| surrealdb::types::RecordId::parse_simple(&s).ok()),
    client_name: session.client_name,
    hardware_hash: session.hardware_hash,
    secret: session.secret,
    authenticated: false,
  };

  let mut pending_request: Option<ClientRequest> = None;

  loop {
    println!("Connecting to server");
    let st = tokio::net::TcpStream::connect(format!("{}:{}", args_server, args_port)).await;
    match st {
      Err(e) => {
        tracing::warn!(
          "Failed to connect to server {}.\nTrying again in 5 seconds",
          e
        );
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
      }
      Ok(stream) => {
        tracing::info!("Connected to server. Setting up codec");
        let mut framed = actix_codec::Framed::new(stream, codec::ServerCodec);

        if let Some(req) = pending_request.take() {
          if let Err(e) = framed.send(req.clone()).await {
            tracing::error!(
              "Failed to send pending request: {}\n Trying again in 5 seconds",
              e
            );
            pending_request = Some(req);
            continue;
          }
        }

        loop {
          tokio::select! {
          msg = framed.next() => {
            if let Some(msg) = msg {
              let authenticated = state.authenticated;

              match msg {
                Ok(msg) => {
                  match (msg, authenticated) {
                    (ServerResponse::Ping, _) => {
                      if let Err(e) = framed.send(ClientRequest::Ping).await {
                        tracing::error!("Failed to queue Ping reply: {}", e);
                      }
                      if !authenticated {
                        tracing::debug!("Attempting to authenticate");
                        let auth_type = utils::derive_auth(state.secret.as_ref(), args_secret.as_ref());
                        match auth_type {
                          Ok(1) => {
                            if let Some(ref secret) = state.secret {
                              if let Some(ref client_id) = state.client_id {
                                if let Err(e) = framed.send(ClientRequest::SigninClient(
                                  secret.clone(),
                                  state.client_name.clone(),
                                  client_id.clone(),
                                  state.hardware_hash.clone(),
                                )).await {
                                  tracing::error!("Failed to queue Signin request: {}", e);
                                }
                              }
                            }
                          }
                          Ok(2) => {
                            if let Some(ref args_secret) = args_secret {
                              if let Err(e) = framed.send(ClientRequest::SignupClient(
                                args_secret.clone(),
                                state.client_name.clone(),
                                state.hardware_hash.clone(),
                              )).await {
                                tracing::error!("Failed to queue Signup request: {}", e);
                              }
                            }
                          }
                          Ok(k) => {
                            tracing::error!("Invalid auth derivation: {}", k);
                            std::process::exit(1);
                          }
                          Err(e) => {
                            tracing::error!("{}", e);
                            std::process::exit(1);
                          }
                        }
                      }
                    }
                    (ServerResponse::Disconnect(reason), _) => {
                      match reason {
                        DisconnectReason::AuthFailed | DisconnectReason::InvalidClientId => {
                          tracing::error!("Authentication failed. Removing stored credentials and quitting. Please restart with a valid --secret.");
                          local_endpoint.query("DELETE session;").await.unwrap();
                          state.authenticated = false;
                          if args_secret.is_none() {
                            std::process::exit(1);
                          }
                        }
                        DisconnectReason::DuplicateClient => {
                          tracing::error!("Duplicate client ID");
                        }
                        DisconnectReason::HeartbeatFailed => {
                          tracing::error!("Heartbeat failed");
                        }
                        DisconnectReason::Unknown(e) => {
                          tracing::error!("Unknown disconnect reason: {}", e);
                        }
                      }
                    }
                    (ServerResponse::SignedIn(token, secret, server_url), _) => {
                      println!("Signed in and received token: {}", &token.grant.key);
                      state.secret = secret;
                      state.authenticated = true;
                      persist_session(&local_endpoint, &state).await;

                      let _ = db_token_tx.send((token, server_url.clone())).await;
                      if let Some(ref cid) = state.client_id {
                        let _ = monitor_cmd_tx.send(MonitorCommand::SetClientId(cid.to_sql())).await;
                      }
                    }
                    (ServerResponse::SignedUp(client_id, token, secret, server_url), _) => {
                      println!("Signed up and received token: {}", &token.grant.key);
                      state.secret = Some(secret);
                      state.client_id = Some(client_id.clone());
                      state.authenticated = true;
                      persist_session(&local_endpoint, &state).await;

                      let _ = db_token_tx.send((token, server_url.clone())).await;
                      let _ = monitor_cmd_tx
                        .send(MonitorCommand::SetClientId(client_id.to_sql())).await;
                    }
                    #[allow(unreachable_patterns)]
                    s => {
                      tracing::debug!("Ignored server response: {:#?}", &s);
                    }
                  }
                }
                Err(e) => {
                  tracing::error!("Failed to receive server message: {}", e);
                }
              }
            } else {
              println!("Server disconnected");
              state.authenticated = false;
              tracing::info!("Server disconnected");
              tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
              break;
            }
          }
          }
        }
      }
    }
  }
}

async fn load_or_create_session(
  local_endpoint: &Surreal<Db>,
) -> crate::db::endpoint::Session {
  match local_endpoint
    .query("SELECT * FROM session ORDER BY updated_at DESC LIMIT 1;")
    .await
  {
    Ok(s) => match s.check() {
      Ok(mut s) => match s.take(1).unwrap_or(None) {
        Some(session) => session,
        None => create_new_session(local_endpoint).await,
      },
      Err(e) => {
        tracing::error!("Failed to check session: {}", e);
        create_new_session(local_endpoint).await
      }
    },
    Err(e) => {
      tracing::error!("Failed to query session: {}\n Creating a new one instead", e);
      create_new_session(local_endpoint).await
    }
  }
}

pub(crate) async fn create_new_session_with_repo(
  repo: &dyn DbOperator<Record = crate::db::endpoint::Session, Input = crate::db::endpoint::SessionData>,
) -> crate::db::endpoint::Session {
  repo.create(crate::db::endpoint::SessionData {
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

async fn create_new_session(
  local_endpoint: &Surreal<Db>,
) -> crate::db::endpoint::Session {
  use crate::db::endpoint::SurrealSessionRepo;
  let repo = SurrealSessionRepo { db: local_endpoint.clone() };
  create_new_session_with_repo(&repo).await
}

pub(crate) async fn persist_session_with_repo(
  repo: &dyn DbOperator<Record = crate::db::endpoint::Session, Input = crate::db::endpoint::SessionData>,
  client_id: Option<surrealdb::types::RecordId>,
  client_name: &str,
  hardware_hash: &str,
  secret: &Option<String>,
) -> String {
  let data = crate::db::endpoint::SessionData {
    client_id: client_id.as_ref().map(|id| id.to_sql()),
    hardware_hash: Some(hardware_hash.to_string()),
    client_name: Some(client_name.to_string()),
    db_addr: None,
    tkn: None,
    secret: secret.clone(),
    groups: vec![],
  };
  repo.create(data).await.unwrap().session_id()
}

async fn persist_session(local_endpoint: &Surreal<Db>, state: &MsgState) {
  use crate::db::endpoint::SurrealSessionRepo;
  let repo = SurrealSessionRepo { db: local_endpoint.clone() };
  let _ = persist_session_with_repo(&repo, state.client_id.clone(), &state.client_name, &state.hardware_hash, &state.secret).await;
}

#[cfg(test)]
mod tests {
  use remex_core::db::DbOperator;
  use remex_core::impl_in_memory_db_operator;
  use surrealdb::types::ToSql;

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

  #[tokio::test]
  async fn persist_session_stores_state() {
    let repo = InMemorySessionRepo::new();
    let cid = surrealdb::types::RecordId::new("client", "test-client");

    let id = super::persist_session_with_repo(
      &repo,
      Some(cid.clone()),
      "test-machine",
      "test-hash-abcd1234",
      &Some("my-secret".to_string()),
    ).await;

    let persisted = repo.read(&id).await.unwrap().expect("session should exist");
    assert_eq!(persisted.client_name, "test-machine");
    assert_eq!(persisted.hardware_hash, "test-hash-abcd1234");
    assert_eq!(persisted.secret, Some("my-secret".to_string()));
    assert_eq!(
      persisted.client_id,
      Some(cid.to_sql()),
      "client_id should be stored as string"
    );
  }

  #[tokio::test]
  async fn persist_session_without_client_id() {
    let repo = InMemorySessionRepo::new();

    let id = super::persist_session_with_repo(
      &repo,
      None,
      "anonymous",
      "hash-0000",
      &None,
    ).await;

    let persisted = repo.read(&id).await.unwrap().expect("session should exist");
    assert_eq!(persisted.client_id, None);
    assert_eq!(persisted.client_name, "anonymous");
    assert_eq!(persisted.hardware_hash, "hash-0000");
  }

  #[tokio::test]
  async fn session_crud_roundtrip() {
    let repo = InMemorySessionRepo::new();

    let created = repo.create(SessionData {
      client_id: Some("client:abc".to_string()),
      client_name: Some("roundtrip-test".to_string()),
      hardware_hash: Some("hash-xyz".to_string()),
      db_addr: None,
      tkn: None,
      secret: Some("s3kr3t".to_string()),
      groups: vec![],
    }).await.unwrap();

    let id = created.session_id();

    let found = repo.read(&id).await.unwrap().expect("session should exist");
    assert_eq!(found.client_name, "roundtrip-test");
    assert_eq!(found.hardware_hash, "hash-xyz");
    assert_eq!(found.secret, Some("s3kr3t".to_string()));

    let updated = repo.update(&id, SessionData {
      client_id: Some("client:abc".to_string()),
      client_name: Some("updated-name".to_string()),
      hardware_hash: Some("hash-xyz".to_string()),
      db_addr: None,
      tkn: None,
      secret: Some("new-secret".to_string()),
      groups: vec![],
    }).await.unwrap();

    assert_eq!(updated.client_name, "updated-name");
    assert_eq!(updated.secret, Some("new-secret".to_string()));

    let refetched = repo.read(&id).await.unwrap().expect("session should still exist");
    assert_eq!(refetched.client_name, "updated-name");

    repo.delete(&id).await.unwrap();
    let gone = repo.read(&id).await.unwrap();
    assert!(gone.is_none(), "session should be gone after delete");
  }
}
