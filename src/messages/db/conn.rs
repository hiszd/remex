use actix::{Addr, AsyncContext};
use actix::{Context, Handler, Message};
use tracing::{error, info, warn};

use crate::core::codec::AuthRequest;
use crate::db;
use crate::endpoint::Endpoint;

#[derive(Message, Clone)]
#[rtype(result = "()")]
pub struct ClientAuth {
  pub identity: Endpoint,
  pub authreq: AuthRequest,
  pub session: Addr<crate::session::RemexSession>,
}

impl Handler<ClientAuth> for db::Db {
  type Result = ();
  fn handle(&mut self, msg: ClientAuth, ctx: &mut Context<Self>) -> Self::Result {
    let m = msg.clone();
    let pool = self.pool.clone();
    match msg.authreq {
      AuthRequest::Secret(secret) => {
        let fut = async move {
          if secret == crate::SECRET {
            //see if there is a client with the machineid from msg.identity
            match db::clients::get_client(&pool, m.identity.machineid.clone()).await {
              Ok(dbclient) => {
                m.session.do_send(super::session::conn::Authenticated {
                  identity: Endpoint {
                    id: Some(dbclient.id.clone()),
                    name: dbclient.name.clone(),
                    machineid: dbclient.machineid.clone(),
                  },
                  secret: dbclient.secret.clone(),
                });
              }
              Err(e) => {
                if let sqlx::Error::RowNotFound = e {
                  warn!("Valid secret, but client not found. Creating new one now");
                  match db::clients::add_client(
                    &pool,
                    m.identity.machineid,
                    db::clients::generate_id(pool.clone()).await.unwrap(),
                    m.identity.name.clone(),
                    db::clients::generate_secret(),
                  )
                  .await
                  {
                    Ok(dbclient) => {
                      tracing::info!(
                        "Client with id: {}, and secret: {} connected",
                        &dbclient.id,
                        &dbclient.secret
                      );
                      m.session.do_send(super::session::conn::Authenticated {
                        identity: Endpoint {
                          id: Some(dbclient.id.clone()),
                          name: dbclient.name.clone(),
                          machineid: dbclient.machineid.clone(),
                        },
                        secret: dbclient.secret.clone(),
                      });
                    }
                    Err(e) => {
                      error!("Error creating client: {}", e);
                      m.session.do_send(super::session::conn::Disconnect {
                        reason: super::DisconnectReason::InvalidClientId,
                      });
                    }
                  }
                }
              }
            }
          }
        };
        let fut = actix::fut::wrap_future::<_, Self>(fut);
        ctx.spawn(fut);
      }
      AuthRequest::IdSecret(id, secret) => {
        let fut = async move {
          //see if there is a client with the machineid from msg.identity
          match db::clients::get_client(&pool, id.clone()).await {
            Ok(dbclient) => {
              if dbclient.secret == secret {
                tracing::info!(
                  "Client with id: {}, and secret: {} connected",
                  &dbclient.id,
                  &dbclient.secret
                );
                m.session.do_send(super::session::conn::Authenticated {
                  identity: Endpoint {
                    id: Some(dbclient.id.clone()),
                    name: dbclient.name.clone(),
                    machineid: dbclient.machineid.clone(),
                  },
                  secret: dbclient.secret.clone(),
                });
              }
            }
            Err(e) => {
              error!("Invalid client id and secret: {}", e);
              m.session.do_send(super::session::conn::Disconnect {
                reason: super::DisconnectReason::InvalidClientId,
              });
            }
          }
        };
        let fut = actix::fut::wrap_future::<_, Self>(fut);
        ctx.spawn(fut);
      }
    }
  }
}
