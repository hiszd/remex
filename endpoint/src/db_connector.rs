use remex_core::db::BearerGrantResponse;
use surrealdb::engine::remote::ws::Client;
use surrealdb::Surreal;
use tokio::sync::{mpsc, watch};

pub async fn run(
  mut token_rx: mpsc::Receiver<(BearerGrantResponse, String)>,
  db_handle_tx: watch::Sender<Option<Surreal<Client>>>,
) {
  while let Some((token, db_url)) = token_rx.recv().await {
    tracing::info!("Connecting to remote database");

    let remote_db: Surreal<Client> = Surreal::init();

    if let Err(e) = remote_db
      .connect::<surrealdb::engine::remote::ws::Ws>(db_url.clone())
      .await
    {
      tracing::error!("Failed to connect to remote database: {}", e);
      let _ = db_handle_tx.send(None);
      continue;
    }

    tracing::info!("Authenticating to remote database");
    if let Err(e) = remote_db
      .signin(surrealdb::opt::auth::Record {
        namespace: "remex".into(),
        database: "remex".into(),
        access: "endpoint".into(),
        params: remex_core::db::BearerToken {
          key: token.grant.key.clone(),
        },
      })
      .await
    {
      tracing::error!("Failed to authenticate with remote database: {}", e);
      let _ = db_handle_tx.send(None);
      continue;
    }

    if let Err(e) = remote_db.use_ns("remex").use_db("remex").await {
      tracing::error!("Failed to select namespace/database: {}", e);
      let _ = db_handle_tx.send(None);
      continue;
    }

    println!("Connected to remote database");
    tracing::info!("Connected to remote database");
    let _ = db_handle_tx.send(Some(remote_db));
  }
}
