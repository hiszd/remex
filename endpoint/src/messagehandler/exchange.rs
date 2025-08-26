use actix_codec::Framed;
use remex_core::codec;
use tokio::net::TcpStream;

use super::Error;
use crate::Context;

#[derive(thiserror::Error, Debug)]
pub enum ExchangeError {
  #[error("Authentication failed")]
  AuthFailed,
}

pub async fn process_exchange_message(
  msg: codec::s2c::Exchange,
  framed: &mut Framed<TcpStream, codec::ServerCodec>,
  c: Context,
) -> Result<Context, Error> {
  let mut ctx = c;
  match msg {
    codec::s2c::Exchange::ExecutorList(list) => {
      tracing::info!("Received executor list: {:?}", &list);
      ctx.executors.update(list).await;
    }
  }
  Ok(ctx)
}
