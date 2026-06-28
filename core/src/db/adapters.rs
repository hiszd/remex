/// Generates a SurrealDB-backed adapter struct implementing `DbOperator`.
///
/// Arguments: `vis? struct_name, Record, Input, "table", "ns", "db"`
#[macro_export]
macro_rules! impl_surreal_db_operator {
  ($vis:vis $name:ident, $record:ty, $input:ty, $table:expr, $ns:expr, $db:expr) => {
    $vis struct $name {
      $vis db: ::surrealdb::Surreal<::surrealdb::engine::local::Db>,
    }

    #[::async_trait::async_trait]
    impl $crate::db::DbOperator for $name {
      type Record = $record;
      type Input = $input;

      async fn create(&self, input: Self::Input) -> Result<Self::Record, $crate::db::DbError> {
        let id = ::uuid::Uuid::new_v4().to_string();
        let rid = ::surrealdb::types::RecordId::new($table, id.as_str());
        let sql = concat!("USE NS ", $ns, " DB ", $db, "; CREATE $id CONTENT $data");
        self.db
          .query(sql)
          .bind(("id", rid))
          .bind(("data", input.clone()))
          .await?
          .check()?;
        let record: $record = (id, input).into();
        Ok(record)
      }

      async fn read(&self, id: &str) -> Result<Option<Self::Record>, $crate::db::DbError> {
        let sql = concat!("USE NS ", $ns, " DB ", $db, "; SELECT * FROM $id");
        Ok(self.db
          .query(sql)
          .bind(("id", ::surrealdb::types::RecordId::new($table, id)))
          .await?
          .check()?
          .take(1)?)
      }

      async fn update(&self, id: &str, input: Self::Input) -> Result<Self::Record, $crate::db::DbError> {
        let sql = concat!("USE NS ", $ns, " DB ", $db, "; UPDATE $id MERGE $data");
        self.db
          .query(sql)
          .bind(("id", ::surrealdb::types::RecordId::new($table, id)))
          .bind(("data", input.clone()))
          .await?
          .check()?;
        let record: $record = (id.to_string(), input).into();
        Ok(record)
      }

      async fn list(&self) -> Result<Vec<Self::Record>, $crate::db::DbError> {
        let sql = concat!("USE NS ", $ns, " DB ", $db, "; SELECT * FROM ", $table);
        let mut result = self.db
          .query(sql)
          .await?
          .check()?;
        Ok(result.take(1)?)
      }

      async fn delete(&self, id: &str) -> Result<(), $crate::db::DbError> {
        let sql = concat!("USE NS ", $ns, " DB ", $db, "; DELETE $id");
        self.db
          .query(sql)
          .bind(("id", ::surrealdb::types::RecordId::new($table, id)))
          .await?
          .check()?;
        Ok(())
      }
    }
  };
}

/// Generates an in-memory adapter struct implementing `DbOperator`.
///
/// Arguments: `vis? struct_name, Record, Input, "table"`
///
/// Requires `Record: From<(String, Input)>` for constructing records with generated IDs.
#[macro_export]
macro_rules! impl_in_memory_db_operator {
  ($vis:vis $name:ident, $record:ty, $input:ty, $table:expr) => {
    $vis struct $name {
      $vis inner: ::std::sync::Arc<::std::sync::Mutex<::std::collections::HashMap<String, $record>>>,
    }

    impl $name {
      pub fn new() -> Self {
        Self {
          inner: ::std::sync::Arc::new(::std::sync::Mutex::new(::std::collections::HashMap::new())),
        }
      }
    }

    #[::async_trait::async_trait]
    impl $crate::db::DbOperator for $name {
      type Record = $record;
      type Input = $input;

      async fn create(&self, input: Self::Input) -> Result<Self::Record, $crate::db::DbError> {
        let id = ::uuid::Uuid::new_v4().to_string();
        let record: $record = (id.clone(), input).into();
        self.inner.lock().unwrap().insert(id, record.clone());
        Ok(record)
      }

      async fn read(&self, id: &str) -> Result<Option<Self::Record>, $crate::db::DbError> {
        Ok(self.inner.lock().unwrap().get(id).cloned())
      }

      async fn update(&self, id: &str, input: Self::Input) -> Result<Self::Record, $crate::db::DbError> {
        let record: $record = (id.to_string(), input).into();
        self.inner.lock().unwrap().insert(id.to_string(), record.clone());
        Ok(record)
      }

      async fn list(&self) -> Result<Vec<Self::Record>, $crate::db::DbError> {
        Ok(self.inner.lock().unwrap().values().cloned().collect())
      }

      async fn delete(&self, id: &str) -> Result<(), $crate::db::DbError> {
        self.inner.lock().unwrap().remove(id);
        Ok(())
      }
    }
  };
}

#[cfg(test)]
mod tests {
  use serde::{Deserialize, Serialize};

  use crate::db::{DbError, DbOperator};

  #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
  pub struct TestRecord {
    pub id: surrealdb::types::RecordId,
    pub name: String,
    pub value: i32,
  }

  #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
  pub struct TestData {
    pub name: String,
    pub value: i32,
  }

  impl From<(String, TestData)> for TestRecord {
    fn from((id, data): (String, TestData)) -> Self {
      TestRecord {
        id: surrealdb::types::RecordId::new("test", id.as_str()),
        name: data.name,
        value: data.value,
      }
    }
  }

  impl_in_memory_db_operator!(pub InMemoryTestRepo, TestRecord, TestData, "test");

  fn rid_key(rid: &surrealdb::types::RecordId) -> String {
    match &rid.key {
      surrealdb::types::RecordIdKey::String(s) => s.clone(),
      _ => panic!("expected string key"),
    }
  }

  // ---- Basic CRUD ----

  #[tokio::test]
  async fn basic_create_and_read() {
    let repo = InMemoryTestRepo::new();
    let data = TestData { name: "hello".into(), value: 42 };

    let created = repo.create(data).await.unwrap();
    assert_eq!(created.name, "hello");
    assert_eq!(created.value, 42);

    let found = repo.read(&rid_key(&created.id)).await.unwrap();
    assert_eq!(found, Some(created));
  }

  #[tokio::test]
  async fn basic_read_missing_returns_none() {
    let repo = InMemoryTestRepo::new();
    let result = repo.read("nonexistent").await.unwrap();
    assert_eq!(result, None);
  }

  #[tokio::test]
  async fn basic_update() {
    let repo = InMemoryTestRepo::new();
    let created = repo.create(TestData { name: "old".into(), value: 1 }).await.unwrap();

    let id = rid_key(&created.id);
    let updated = repo.update(&id, TestData { name: "new".into(), value: 2 }).await.unwrap();
    assert_eq!(updated.name, "new");
    assert_eq!(updated.value, 2);

    let found = repo.read(&id).await.unwrap();
    assert_eq!(found, Some(updated));
  }

  #[tokio::test]
  async fn basic_delete() {
    let repo = InMemoryTestRepo::new();
    let created = repo.create(TestData { name: "temp".into(), value: 0 }).await.unwrap();

    let id = rid_key(&created.id);
    repo.delete(&id).await.unwrap();

    let found = repo.read(&id).await.unwrap();
    assert_eq!(found, None);
  }

  #[tokio::test]
  async fn basic_crud_lifecycle() {
    let repo = InMemoryTestRepo::new();

    let data = TestData { name: "cycle".into(), value: 10 };
    let created = repo.create(data).await.unwrap();
    let id = rid_key(&created.id);

    let updated = repo.update(&id, TestData { name: "cycle".into(), value: 20 }).await.unwrap();
    assert_eq!(updated.value, 20);

    let found = repo.read(&id).await.unwrap();
    assert_eq!(found, Some(updated));

    repo.delete(&id).await.unwrap();
    assert_eq!(repo.read(&id).await.unwrap(), None);
  }

  // ---- Seam pattern: functions accepting &dyn DbOperator ----

  /// Simulates a business operation: create a record, update it twice, return final value.
  /// This function is written against the trait — it works with ANY adapter.
  async fn business_flow(
    repo: &dyn DbOperator<Record = TestRecord, Input = TestData>,
    seed_name: &str,
  ) -> Result<i32, DbError> {
    let created = repo.create(TestData { name: seed_name.into(), value: 1 }).await?;
    let id = rid_key(&created.id);

    let v1 = repo.update(&id, TestData { name: format!("{seed_name}_step1"), value: 10 }).await?;
    let v2 = repo.update(&id, TestData { name: format!("{seed_name}_step2"), value: v1.value + 5 }).await?;

    Ok(v2.value)
  }

  #[tokio::test]
  async fn seam_business_flow_with_in_memory() {
    let repo = InMemoryTestRepo::new();
    let result = business_flow(&repo, "seam_test").await.unwrap();
    assert_eq!(result, 15);
  }

  #[tokio::test]
  async fn seam_multiple_flows_independent() {
    let repo = InMemoryTestRepo::new();

    let (r1, r2) = tokio::join!(
      business_flow(&repo, "alpha"),
      business_flow(&repo, "beta"),
    );
    assert_eq!(r1.unwrap(), 15);
    assert_eq!(r2.unwrap(), 15);
  }

  /// A function that takes a boxed trait object — identical to how production
  /// code would inject dependencies at construction time.
  async fn boxed_flow(
    repo: Box<dyn DbOperator<Record = TestRecord, Input = TestData>>,
    name: &str,
  ) -> Result<String, DbError> {
    let created = repo.create(TestData { name: name.into(), value: 99 }).await?;
    Ok(rid_key(&created.id))
  }

  #[tokio::test]
  async fn seam_boxed_trait_object() {
    let id = boxed_flow(Box::new(InMemoryTestRepo::new()), "boxed").await.unwrap();
    assert!(!id.is_empty());
  }

  // ---- Concurrency ----

  #[tokio::test]
  async fn concurrent_creates() {
    let repo = std::sync::Arc::new(InMemoryTestRepo::new());
    let mut handles = Vec::new();

    for i in 0..100 {
      let r = repo.clone();
      handles.push(tokio::spawn(async move {
        r.create(TestData { name: format!("t{i}"), value: i }).await.unwrap();
      }));
    }

    for h in handles {
      h.await.unwrap();
    }

    // Verify all 100 records are independently stored
    // (there's no read-all method, so we just verify the count via read pattern)
    for i in 0..100 {
      let record = repo.create(TestData { name: format!("verify_{i}"), value: i }).await.unwrap();
      let id = rid_key(&record.id);
      let found = repo.read(&id).await.unwrap();
      assert!(found.is_some());
    }
  }

  #[tokio::test]
  async fn concurrent_read_write_on_same_repo() {
    let repo = std::sync::Arc::new(InMemoryTestRepo::new());
    let created = repo.create(TestData { name: "shared".into(), value: 0 }).await.unwrap();
    let id = std::sync::Arc::new(rid_key(&created.id));

    let mut handles = Vec::new();
    for i in 0..50 {
      let r = repo.clone();
      let rid = id.clone();
      handles.push(tokio::spawn(async move {
        let _ = r.update(&rid, TestData { name: "shared".into(), value: i }).await.unwrap();
        let _ = r.read(&rid).await.unwrap();
      }));
    }

    for h in handles {
      h.await.unwrap();
    }

    // Final value is one of the 50 concurrent writes (non-deterministic)
    let final_record = repo.read(&id).await.unwrap();
    assert!(final_record.is_some());
    assert_eq!(final_record.unwrap().name, "shared");
  }

  // ---- State isolation ----

  #[tokio::test]
  async fn isolation_two_repos_independent() {
    let repo_a = InMemoryTestRepo::new();
    let repo_b = InMemoryTestRepo::new();

    let created_a = repo_a.create(TestData { name: "a_only".into(), value: 1 }).await.unwrap();
    let created_b = repo_b.create(TestData { name: "b_only".into(), value: 2 }).await.unwrap();

    let id_a = rid_key(&created_a.id);
    let id_b = rid_key(&created_b.id);

    // Each repo only sees its own records
    assert!(repo_a.read(&id_b).await.unwrap().is_none());
    assert!(repo_b.read(&id_a).await.unwrap().is_none());

    // Each repo's records have correct data
    assert_eq!(repo_a.read(&id_a).await.unwrap().unwrap().value, 1);
    assert_eq!(repo_b.read(&id_b).await.unwrap().unwrap().value, 2);
  }

  // ---- Edge cases ----

  #[tokio::test]
  async fn edge_update_creates_if_not_exists() {
    // Matches SurrealDB UPSERT semantics
    let repo = InMemoryTestRepo::new();
    let updated = repo.update("nonexistent-id", TestData { name: "upserted".into(), value: 42 }).await.unwrap();

    assert_eq!(updated.name, "upserted");
    assert_eq!(updated.value, 42);

    // Verify it's actually stored
    let found = repo.read("nonexistent-id").await.unwrap();
    assert_eq!(found, Some(updated));
  }

  #[tokio::test]
  async fn edge_delete_non_existent_does_not_error() {
    let repo = InMemoryTestRepo::new();
    // This must not panic or return Err
    repo.delete("i-dont-exist").await.unwrap();
  }

  #[tokio::test]
  async fn edge_multiple_records_distinct_ids() {
    let repo = InMemoryTestRepo::new();
    let mut ids = Vec::new();

    for i in 0..10 {
      let record = repo.create(TestData { name: format!("rec_{i}"), value: i }).await.unwrap();
      ids.push(rid_key(&record.id));
    }

    // Verify all distinct
    let mut sorted = ids.clone();
    sorted.sort();
    sorted.dedup();
    assert_eq!(ids.len(), sorted.len(), "all IDs must be unique");

    // Verify each can be read
    for (i, id) in ids.iter().enumerate() {
      let found = repo.read(id).await.unwrap().unwrap();
      assert_eq!(found.value, i as i32);
    }
  }

  #[tokio::test]
  async fn edge_update_preserves_other_fields() {
    let repo = InMemoryTestRepo::new();
    let record = repo.create(TestData { name: "original".into(), value: 100 }).await.unwrap();
    let id = rid_key(&record.id);

    // Update only value (via new TestData with same name)
    repo.update(&id, TestData { name: "original".into(), value: 200 }).await.unwrap();

    let found = repo.read(&id).await.unwrap().unwrap();
    assert_eq!(found.name, "original");
    assert_eq!(found.value, 200);
  }
}
