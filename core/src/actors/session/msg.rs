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
use tracing::info;

use crate::{
  codec::{
    DisconnectReason,
    ServerResponse,
  },
  db::get_endpoint_bearer_token,
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
    self.framed.write(ServerResponse::Disconnect(disc.reason));
  }
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct SignupClient {
  /// SignupClient (Server Secret, Client_Name, Hardware_Hash)
  pub iden: (String, String, String),
  pub db: Option<surrealdb::Surreal<surrealdb::engine::any::Any>>,
  pub server_secret: String,
}
impl Handler<SignupClient> for super::RemexSession {
  type Result = ();
  fn handle(&mut self, msg: SignupClient, ctx: &mut Context<Self>) -> Self::Result {
    use crate::db::model::clients::Client;

    let db = msg.db.clone();
    let server_secret = msg.server_secret.clone();
    let (secret, name, hardware_hash) = msg.iden;

    async move {
      info!("client attempting to connect with server secret: {}", &secret);
      if secret != server_secret {
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
          client_secret,
        ))
      } else {
        Err(anyhow::anyhow!("Client not found 1"))
      }
    }
    .into_actor(self)
    .map(|result, act, _ctx| match result {
      Ok((client_id, name, token, secret)) => {
        act.client_id = Some(client_id.clone());
        act.name = Some(name.clone());
        act.authenticated = true;
        tracing::info!("client {} authenticated. sent token {}", &name, &token.grant.key);
        act
          .framed
          .write(ServerResponse::SignedUp(client_id, token, secret));
      }
      Err(e) => {
        tracing::error!("client authentication error: {}", e);
        act
          .framed
          .write(ServerResponse::Disconnect(DisconnectReason::AuthFailed));
        act.framed.close();
      }
    })
    .spawn(ctx);
  }
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct SigninClient {
  // SigninClient (Client_Secret, Client_Name, Client_Id, Hardware_Hash)
  pub iden: (String, String, surrealdb::types::RecordId, String),
  pub db: Option<surrealdb::Surreal<surrealdb::engine::any::Any>>,
}
impl Handler<SigninClient> for super::RemexSession {
  type Result = ();
  fn handle(&mut self, msg: SigninClient, ctx: &mut Context<Self>) -> Self::Result {
    use crate::db::model::clients::Client;

    let db = msg.db.clone();
    let (client_secret, client_name, client_id, hardware_hash) = msg.iden;

    async move {
      info!(
        "Client attempting to connect with client secret: {}, name: {} id: {:?}",
        &client_secret, &client_name, &client_id,
      );

      let d = db
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("database not available"))?;

      // WARN: This might throw an error if the client doesn't exist instead of returning None
      let client: Option<Client> = d.query("USE NS remex DB remex; SELECT * FROM client WHERE id = $id AND secret = crypto::argon2::compare($secret, secret) AND hardware_hash = $hardware_hash;")
        .bind(("id", client_id))
        .bind(("secret", client_secret.clone()))
        .bind(("hardware_hash", hardware_hash))
        .await?.check()?.take(1)?;

      if let Some(c) = client {
        // Enroll the client in the database, and store the secret.
        let mut response = d
          .query("USE NS remex DB remex; UPSERT client CONTENT $data;")
          .bind(("data", crate::db::model::clients::ClientData {
            client_name: c.client_name,
            hardware_hash: c.hardware_hash,
            secret: client_secret.clone(),
          }))
          .await
          .unwrap()
          .check()
          .unwrap();
        let cli: Option<Client> = response.take(1)?;
        if let Some(cl) = cli {
          Ok((
            cl.id.clone(),
            cl.client_name,
            get_endpoint_bearer_token(cl.id, d)
              .await
              .expect("token not found")
              .expect("token not found"),
            // TODO: invalidate the client secret at some point and return the new secret here
            None::<String>,
          ))
        } else {
          Err(anyhow::anyhow!("Client not found"))
        }
      } else {
        Err(anyhow::anyhow!("Client not found"))
      }
    }
      .into_actor(self)
      .map(|result, act, _ctx| match result {
        Ok((id, name, t, secret)) => {
          act.client_id = Some(id.clone());
          act.name = Some(name.clone());
          act.authenticated = true;
          tracing::info!("client {} authenticated. sent token {}", &name, &t.grant.key);
          act
            .framed
            .write(ServerResponse::SignedIn(t, secret));
        }
        Err(e) => {
          tracing::error!("client authentication error: {}", e);
          act
            .framed
            .write(ServerResponse::Disconnect(
              DisconnectReason::AuthFailed,
            ));
          act.framed.close();
        }
      })
      .spawn(ctx);
  }
}
