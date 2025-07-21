use actix::{AsyncContext, Context, Handler, Message};

use super::{server, session};
use crate::core::codec::DisconnectReason;
use crate::db::{clients, Db};
use crate::endpoint::Endpoint;

#[derive(Message)]
#[rtype(result = "Vec<String>")]
pub struct GetLogs {}
impl Handler<GetLogs> for Db {
  type Result = Vec<String>;
  fn handle(&mut self, _msg: GetLogs, _ctx: &mut Context<Self>) -> Self::Result {
    let _lgs = futures::executor::block_on(async {
      sqlx::query("SELECT * FROM logs").fetch_all(&self.pool).await.unwrap()
    });
    vec!["bob".to_owned()]
  }
}

#[derive(Debug)]
pub struct NewClient {
  pub identity: Endpoint,
  pub session: actix::Addr<crate::session::RemexSession>,
  pub server: actix::Addr<crate::server::Server>,
}
impl Message for NewClient {
  type Result = Result<(), anyhow::Error>;
}
impl Handler<NewClient> for Db {
  type Result = Result<(), anyhow::Error>;
  fn handle(&mut self, msg: NewClient, ctx: &mut Context<Self>) -> Self::Result {
    let identity = msg.identity.clone();
    let session = msg.session.clone();
    let server = msg.server.clone();
    let pool = self.pool.clone();
    let futr = Box::pin(async move {
      let query;
      if identity.id.is_none() {
        tracing::info!("Generating new id");
        let id = clients::generate_id(pool.clone()).await.unwrap();
        tracing::info!("Generated id: {}", id);
        tracing::info!("Generating new secret");
        let secret = clients::generate_secret();
        tracing::info!("Generated secret: {}", secret);
        query = clients::add_client(pool, id, identity.name.clone(), secret.clone()).await;
      } else {
        tracing::info!("Using existing id {}", identity.id.clone().unwrap());
        query = clients::get_client(&pool, identity.id.clone().unwrap()).await;
      }
      match query {
        Ok(client) => {
          tracing::info!(
            "Client with id: {}, and secret: {}  connected",
            &client.id,
            &client.secret
          );
          let new_identity = identity.clone().merge(Endpoint {
            id: Some(client.id.clone()),
            name: client.name.clone(),
            machineid: identity.machineid.clone(),
          });
          server.do_send(server::conn::DbClientIdentified {
            identity: new_identity.clone(),
            secret: client.secret.clone(),
            addr: session.clone(),
          });
          session.do_send(session::conn::Identified {
            identity: new_identity.clone(),
            secret: client.secret.clone(),
            temp: false,
          });
        }
        Err(e) => match e {
          sqlx::Error::RowNotFound => {
            tracing::error!("client not found");
            session.do_send(session::conn::Disconnect {
              reason: DisconnectReason::InvalidClientId,
            });
          }
          _ => {}
        },
      }
    });
    let fut = actix::fut::wrap_future::<_, Self>(futr);
    ctx.spawn(fut);
    Ok(())
  }
}

#[derive(Message)]
#[rtype(result = "()")]
struct NewLog {
  client: String,
  message: String,
  time_logged: chrono::NaiveDateTime,
}
impl Handler<NewLog> for Db {
  type Result = ();
  fn handle(&mut self, msg: NewLog, _ctx: &mut Context<Self>) {
    futures::executor::block_on(self.new_log(&msg.client, &msg.message, msg.time_logged));
  }
}
