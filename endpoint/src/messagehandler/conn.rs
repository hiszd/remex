use actix_codec::Framed;
use futures::SinkExt;
use remex_core::codec::{
  self,
  c2s::C2S,
  AuthRequest,
};
use tokio::net::TcpStream;
use tracing::info;

use super::Error;
use crate::{
  fs::identity::StoredEndpoint,
  Context,
};

#[derive(thiserror::Error, Debug)]
pub enum ConnError {
  #[error("Authentication failed")]
  AuthFailed,
  #[error("Invalid client id")]
  InvalidClientId,
  #[error("Invalid secret")]
  InvalidSecret,
}

pub async fn process_conn_message(
  msg: codec::s2c::Conn,
  framed: &mut Framed<TcpStream, codec::ServerCodec>,
  c: Context,
) -> Result<Context, Error> {
  let mut ctx = c;
  match msg {
    codec::s2c::Conn::Command(ref cmd) => {
      info!("command: {cmd}");
    }
    codec::s2c::Conn::Message(ref msg) => {
      info!("message: {msg}");
    }
    // respond to the server asking us to identify
    codec::s2c::Conn::Identify => {
      let authreq: AuthRequest = if ctx.identity.secret.is_some() {
        let identity = ctx.identity.clone();
        info!(
          "Using Id and Secret: {}, {}",
          ctx.identity.machineid.clone(),
          ctx.identity.secret.clone().unwrap()
        );
        AuthRequest::IdSecret(identity.machineid, identity.secret.unwrap())
      } else {
        info!("Using just Secret");
        AuthRequest::Secret(remex_core::SECRET.to_string())
      };
      framed
        .send(C2S::Conn(codec::c2s::Conn::Identify(ctx.identity.clone().into(), authreq)))
        .await?;
    }
    codec::s2c::Conn::Authenticated(epnt, secret) => {
      info!("Session authenticated. Id: {:?}, Name: {}, Secret: {}", &epnt.id, &epnt.name, &secret);
      ctx.identity = StoredEndpoint::from(epnt.clone());
      ctx.identity.secret = Some(secret);
      ctx.connected = true;
      crate::fs::identity::save_identity(ctx.identity.clone())?;
      framed
        .send(C2S::Exchange(codec::c2s::Exchange::SendConfiguration))
        .await?;
      // framed
      //   .send(C2S::Conn(codec::c2s::Conn::Message("Successfully authenticated".to_string())))
      //   .await?;
    }
    codec::s2c::Conn::Disconnect(reason) => {
      match reason {
        codec::DisconnectReason::InvalidClientId => {
          // if the client id is invalid, then remove it so you can get a new
          // one
          crate::fs::identity::reset_identity()?;
          return Err(ConnError::InvalidClientId.into());
        }
        codec::DisconnectReason::AuthFailed => {
          return Err(ConnError::AuthFailed.into());
        }
        codec::DisconnectReason::InvalidSecret => {
          return Err(ConnError::InvalidSecret.into());
        }
      }
    }

    // respond to pings with a "pong"
    codec::s2c::Conn::Ping => {
      framed.send(C2S::Conn(codec::c2s::Conn::Ping)).await?;
    }

    _ => {
      eprintln!("{msg:?}");
    }
  }
  Ok(ctx)
}
