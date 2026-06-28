use surrealdb::{
  engine::any::Any,
  Surreal,
};
use tokio::sync::{
  mpsc,
  watch,
};

pub async fn run(
  mut client_id_rx: mpsc::Receiver<String>,
  mut db_handle_rx: watch::Receiver<Option<(Surreal<Any>, String)>>,
) {
  let mut client_id: Option<String> = None;
  let mut db: Option<Surreal<Any>> = None;
  let mut auth_token: Option<String> = None;

  let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));
  interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

  loop {
    tokio::select! {
      cid = client_id_rx.recv() => {
        match cid {
          Some(id) => {
            tracing::info!("Heartbeat task received client_id: {id}");
            client_id = Some(id);
          }
          None => {
            tracing::warn!("client_id channel closed, stopping heartbeat task");
            break;
          }
        }
      }
      result = db_handle_rx.changed() => {
        match result {
          Ok(()) => {
            let entry = db_handle_rx.borrow().clone();
            if let Some((ref handle, ref token)) = entry {
              db = Some(handle.clone());
              auth_token = Some(token.clone());
              tracing::info!("Heartbeat task received db handle");
            } else {
              tracing::info!("Heartbeat task received cleared db handle");
              db = None;
              auth_token = None;
            }
          }
          Err(_) => {
            tracing::warn!("db_handle channel closed, stopping heartbeat task");
            break;
          }
        }
      }
      _ = interval.tick() => {
        tracing::trace!("Heartbeat tick fired (client_id={}, db={})",
          client_id.is_some(),
          db.is_some(),
        );
        if let Some(ref cid) = client_id {
          if db.is_some() && auth_token.is_some() {
            let cid = cid.clone();
            let db = db.clone().unwrap();
            let tkn = auth_token.clone().unwrap();
              tokio::spawn(async move {
                if let Err(e) = db.authenticate(tkn).await {
                  tracing::warn!("Heartbeat authenticate failed: {e}");
                  return;
                }
                match surrealdb::types::RecordId::parse_simple(&cid) {
                  Ok(rid) => {
                    match db
                      .query("UPDATE $id SET last_seen = time::now()")
                      .bind(("id", rid))
                      .await
                    {
                      Ok(mut response) => {
                        let updated: Vec<serde_json::Value> = match response.take(0) {
                          Ok(v) => v,
                          Err(e) => {
                            tracing::warn!("UPDATE failed on server: {e}");
                            vec![]
                          }
                        };
                        if updated.is_empty() {
                          tracing::warn!(
                            "UPDATE last_seen for {cid} matched zero records"
                          );
                        }
                      }
                      Err(e) => tracing::warn!("Failed to send UPDATE: {e}"),
                    }
                  }
                  Err(e) => tracing::warn!("Invalid client_id {cid}: {e}"),
                }
              });
          }
        }
      }
    }
  }
}

#[cfg(test)]
mod heartbeat_tests {
  use surrealdb::{
    engine::local::SurrealKv,
    types::{
      Datetime,
      RecordId,
      SurrealValue,
    },
    Surreal,
  };

  #[derive(serde::Deserialize, SurrealValue)]
  struct TestClient {
    id: RecordId,
    client_name: String,
    hardware_hash: Option<String>,
    last_seen: Option<Datetime>,
  }

  #[tokio::test]
  async fn test_update_last_seen_sets_the_field() {
    let dir = std::env::temp_dir().join("remex-heartbeat-test");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let db_path = dir.join("test.db");

    let db = Surreal::new::<SurrealKv>(db_path).await.unwrap();
    db.use_ns("remex").use_db("remex").await.unwrap();

    db.query(
      "
      DEFINE TABLE client SCHEMAFULL;
      DEFINE FIELD client_name ON TABLE client TYPE string;
      DEFINE FIELD last_seen ON TABLE client TYPE option<datetime>;
      ",
    )
    .await
    .unwrap()
    .check()
    .unwrap();

    db.query("CREATE client:test SET client_name = 'test'")
      .await
      .unwrap()
      .check()
      .unwrap();

    let mut result = db.query("SELECT * FROM client:test").await.unwrap();
    let before: Vec<TestClient> = result.take(0).unwrap();
    assert_eq!(before.len(), 1);
    assert!(before[0].last_seen.is_none());

    let rid = RecordId::parse_simple("client:test").unwrap();
    db.query("UPDATE $id SET last_seen = time::now()")
      .bind(("id", rid))
      .await
      .unwrap()
      .check()
      .unwrap();

    let mut result = db.query("SELECT * FROM client:test").await.unwrap();
    let after: Vec<TestClient> = result.take(0).unwrap();
    assert_eq!(after.len(), 1);
    assert!(after[0].last_seen.is_some(), "last_seen should be Some after UPDATE");

    let _ = std::fs::remove_dir_all(&dir);
  }

  #[tokio::test]
  async fn test_heartbeat_update_returns_empty_for_nonexistent_record() {
    let dir = std::env::temp_dir().join("remex-heartbeat-noexist-test");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let db_path = dir.join("test.db");

    let db = Surreal::new::<SurrealKv>(db_path).await.unwrap();
    db.use_ns("remex").use_db("remex").await.unwrap();

    db.query(
      "
      DEFINE TABLE client SCHEMAFULL;
      DEFINE FIELD client_name ON TABLE client TYPE string;
      DEFINE FIELD last_seen ON TABLE client TYPE option<datetime>;
      ",
    )
    .await
    .unwrap()
    .check()
    .unwrap();

    // Update a record that doesn't exist (simulates stale client_id)
    let rid = RecordId::parse_simple("client:nonexistent").unwrap();
    let mut response = db
      .query("UPDATE $id SET last_seen = time::now()")
      .bind(("id", rid))
      .await
      .unwrap();

    let updated: Vec<serde_json::Value> = response.take(0).unwrap();
    assert!(updated.is_empty(), "UPDATE of nonexistent record should return empty result");
    let _ = std::fs::remove_dir_all(&dir);
  }

  #[tokio::test]
  async fn test_heartbeat_update_returns_record_for_existing_record() {
    let dir = std::env::temp_dir().join("remex-heartbeat-existing-test");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let db_path = dir.join("test.db");

    let db = Surreal::new::<SurrealKv>(db_path).await.unwrap();
    db.use_ns("remex").use_db("remex").await.unwrap();

    db.query(
      "
      DEFINE TABLE client SCHEMAFULL;
      DEFINE FIELD client_name ON TABLE client TYPE string;
      DEFINE FIELD last_seen ON TABLE client TYPE option<datetime>;
      ",
    )
    .await
    .unwrap()
    .check()
    .unwrap();

    db.query("CREATE client:test SET client_name = 'test'")
      .await
      .unwrap()
      .check()
      .unwrap();

    let rid = RecordId::parse_simple("client:test").unwrap();
    let mut response = db
      .query("UPDATE $id SET last_seen = time::now()")
      .bind(("id", rid))
      .await
      .unwrap();

    let updated: Vec<serde_json::Value> = response.take(0).unwrap();
    assert_eq!(updated.len(), 1, "UPDATE should return the updated record");
    let _ = std::fs::remove_dir_all(&dir);
  }
}
