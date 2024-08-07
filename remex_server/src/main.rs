//SERVER
use remex_core::db;
use tokio::net::TcpListener;
use tracing_subscriber;

mod args;
mod conn;
mod endpoint;

#[derive(Debug, Clone)]
pub enum ERROR {
  InvalidSecret,
  InvalidPacket,
  InvalidLength,
  NotConnected,
  NotEnoughPackets,
}

impl From<ERROR> for String {
  fn from(value: ERROR) -> Self {
    match value {
      ERROR::InvalidSecret => "invalid secret".to_string(),
      ERROR::InvalidPacket => "invalid packet".to_string(),
      ERROR::InvalidLength => "invalid length".to_string(),
      ERROR::NotConnected => "not connected".to_string(),
      ERROR::NotEnoughPackets => "not enough packets".to_string(),
    }
  }
}

const ADDRESS: &str = "127.0.0.1:4269";

const SECRET: &str = "tZs3U%hqY^o$&*y%4HcF8&RyAKevUbZnkTsrjCzPGxfare3Yn9c7shVZETfPDPUc8xR%N38a!TL%2$WbkFhZqmH#jvw&d3^mryPD8Y8TqHoJHwyKSTJeQB7vK7QkW#&B";

#[tokio::main]
async fn main() {
  tracing_subscriber::fmt::init();
  let listener = TcpListener::bind(ADDRESS).await.unwrap();
  let path_str = if !cfg!(debug_assertions) {
    "db/prod.db"
  } else {
    "db/dev.db"
  };
  let db = db::Db::new(path_str.to_string()).await;
  db.migrate().await;

  loop {
    let (stream, _) = listener.accept().await.unwrap();
    tokio::spawn(async move {
      let mut conn = conn::Conn::new(stream, SECRET.to_string());
      conn.process().await;
    });
  }
}
