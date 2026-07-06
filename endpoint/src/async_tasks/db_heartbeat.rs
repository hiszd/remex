use std::time::Duration;
use actix::prelude::*;
use surrealdb::{engine::any::Any, Surreal};
use tracing;

pub struct HeartbeatActor {
    client_id: Option<String>,
    remote_db: Option<Surreal<Any>>,
    interval: Duration,
}

impl HeartbeatActor {
    pub fn new() -> Self {
        HeartbeatActor {
            client_id: None,
            remote_db: None,
            interval: Duration::from_secs(60),
        }
    }
}

#[derive(Message)]
#[rtype(result = "()")]
struct HeartbeatTick;

impl Actor for HeartbeatActor {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        ctx.notify_later(HeartbeatTick, self.interval);
    }
}

impl actix::Supervised for HeartbeatActor {
    fn restarting(&mut self, ctx: &mut Context<Self>) {
        tracing::info!("HeartbeatActor: restarting");
        self.client_id = None;
        self.remote_db = None;
        // Re-schedule the heartbeat tick (started() is not called on restart)
        ctx.notify_later(HeartbeatTick, self.interval);
    }
}

impl Handler<crate::async_tasks::ConnectionReady> for HeartbeatActor {
    type Result = ();

    fn handle(&mut self, msg: crate::async_tasks::ConnectionReady, _ctx: &mut Self::Context) {
        self.remote_db = msg.db;
        self.client_id = msg.client_id;
        tracing::info!(
            "Heartbeat actor received connection (db={}, client_id={})",
            self.remote_db.is_some(),
            self.client_id.is_some(),
        );
    }
}

impl Handler<HeartbeatTick> for HeartbeatActor {
    type Result = ();

    fn handle(&mut self, _msg: HeartbeatTick, ctx: &mut Self::Context) {
        if let Some(ref cid) = self.client_id {
            if let Some(ref db) = self.remote_db {
                let cid = cid.clone();
                let db = db.clone();
                tokio::spawn(async move {
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
                                        tracing::warn!("UPDATE last_seen for {cid} matched zero records");
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
        ctx.notify_later(HeartbeatTick, self.interval);
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
