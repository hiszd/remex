use actix::{
  fut::{
    ActorFutureExt,
    WrapFuture,
  },
  Context,
  ContextFutureSpawner,
  Handler,
  Message,
};
use surrealdb::types::{
  SurrealValue,
  ToSql,
};
use tracing::info;

use crate::{
  codec::{
    ConnectionResponse,
    DisconnectReason,
    EndpointSigninCreds,
    EndpointSignupCreds,
    ServerResponse,
  },
  db::{
    get_endpoint_bearer_token,
    BearerGrantResponse,
  },
  utils::generate_secret,
};

#[derive(Message)]
#[rtype(result = "()")]
pub struct Disconnect {
  pub reason: crate::codec::DisconnectReason,
}
impl Handler<Disconnect> for super::RemexSession {
  type Result = ();
  fn handle(&mut self, disc: Disconnect, _: &mut Context<Self>) -> Self::Result {
    tracing::info!("Sending disconnect to peer");
    self
      .framed
      .write(ServerResponse::ConnectionResponse(ConnectionResponse::Disconnect(disc.reason)));
  }
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct Authenticate {
  pub iden: crate::codec::IdentifyType,
  pub db: Option<surrealdb::Surreal<surrealdb::engine::any::Any>>,
  pub server_secret: String,
}
impl Handler<Authenticate> for super::RemexSession {
  type Result = ();
  fn handle(&mut self, auth: Authenticate, ctx: &mut Context<Self>) -> Self::Result {
    use surrealdb::opt::auth::Record;

    use crate::{
      codec::IdentifyType,
      db::model::clients::Client,
    };

    let db = auth.db.clone();
    let server_secret = auth.server_secret.clone();
    let iden = auth.iden;

    async move {
      match iden {
        IdentifyType::Secret(sec, name, hardware_hash) => {
          info!("client attempting to connect with server secret: {}", &sec);
          if sec != server_secret {
            info!("secret mismatch");
            return Err(anyhow::anyhow!("secret mismatch"));
          }
          info!("secret match for client: {}, {}", &name, &hardware_hash);

          let d = db
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("database not available"))?;

          // Generate the client secret to be put in the database, and sent along to the client.
          let client_secret = generate_secret(true);

          // Enroll the client in the database, and store the secret.
          let mut response = d
            .query("USE NS remex DB remex; UPSERT client CONTENT $data;")
            .bind(("data", crate::db::model::clients::ClientData {
              client_name: name,
              hardware_hash,
              secret: client_secret.clone(),
            }))
            .await
            .unwrap()
            .check()
            .unwrap();
          let client: Option<Client> = response.take(1)?;
          if let Some(c) = client {
            Ok((
              c.id.clone(),
              c.client_name,
              get_endpoint_bearer_token(c.id, d)
                .await
                .expect("token not found")
                .expect("token not found"),
              c.secret,
            ))
          } else {
            Err(anyhow::anyhow!("Client not found"))
          }
        }
        IdentifyType::ClientSecret(credential, client_name, client_id, hardware_hash) => {
          info!(
            "Client attempting to connect with client secret: {}, name: {} id: {:?}",
            &credential, &client_name, &client_id,
          );

          let d = db
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("database not available"))?;

          // Enroll the client in the database, and store the secret.
          let mut response = d
            .query("USE NS remex DB remex; UPSERT client CONTENT $data;")
            .bind(("data", crate::db::model::clients::ClientData {
              client_name,
              hardware_hash,
              secret: credential.clone(),
            }))
            .await
            .unwrap()
            .check()
            .unwrap();
          let client: Option<Client> = response.take(1)?;
          if let Some(c) = client {
            Ok((
              c.id.clone(),
              c.client_name,
              get_endpoint_bearer_token(c.id, d)
                .await
                .expect("token not found")
                .expect("token not found"),
              credential,
            ))
          } else {
            Err(anyhow::anyhow!("Client not found"))
          }
        }
      }
    }
    .into_actor(self)
    .map(|result, act, _ctx| match result {
      Ok((client_id, name, token, _credential)) => {
        act.client_id = Some(client_id.clone());
        act.name = Some(name.clone());
        act.authenticated = true;
        tracing::info!("client {} authenticated. sent token {}", &name, &token.grant.key);
        act
          .framed
          .write(ServerResponse::ConnectionResponse(ConnectionResponse::Authenticated(
            client_id, token,
          )));
      }
      Err(e) => {
        tracing::error!("client authentication error: {}", e);
        act
          .framed
          .write(ServerResponse::ConnectionResponse(ConnectionResponse::Disconnect(
            DisconnectReason::AuthFailed,
          )));
        act.framed.close();
      }
    })
    .spawn(ctx);
  }
}
