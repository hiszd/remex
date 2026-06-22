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
        use $crate::db::DbError;
        let sql = concat!("USE NS ", $ns, " DB ", $db, "; CREATE ", $table, " CONTENT $data");
        let mut result = self.db
          .query(sql)
          .bind(("data", input))
          .await?
          .check()?;
        let record: Option<$record> = result.take(1)?;
        record.ok_or_else(|| {
          DbError::OperationFailed(concat!("Failed to create ", $table).into())
        })
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
        use $crate::db::DbError;
        let sql = concat!("USE NS ", $ns, " DB ", $db, "; UPDATE $id CONTENT $data");
        let mut result = self.db
          .query(sql)
          .bind(("id", ::surrealdb::types::RecordId::new($table, id)))
          .bind(("data", input))
          .await?
          .check()?;
        let record: Option<$record> = result.take(1)?;
        record.ok_or_else(|| {
          DbError::OperationFailed(concat!("Failed to update ", $table).into())
        })
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

  use crate::db::DbOperator;

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

  #[tokio::test]
  async fn test_create_and_read() {
    let repo = InMemoryTestRepo::new();
    let data = TestData { name: "hello".into(), value: 42 };

    let created = repo.create(data).await.unwrap();
    assert_eq!(created.name, "hello");
    assert_eq!(created.value, 42);

    let found = repo.read(&rid_key(&created.id)).await.unwrap();
    assert_eq!(found, Some(created));
  }

  #[tokio::test]
  async fn test_read_missing_returns_none() {
    let repo = InMemoryTestRepo::new();
    let result = repo.read("nonexistent").await.unwrap();
    assert_eq!(result, None);
  }

  #[tokio::test]
  async fn test_update() {
    let repo = InMemoryTestRepo::new();
    let created = repo.create(TestData { name: "old".into(), value: 1 }).await.unwrap();

    let id = rid_key(&created.id);
    let updated = repo.update(
      &id,
      TestData { name: "new".into(), value: 2 },
    ).await.unwrap();

    assert_eq!(updated.name, "new");
    assert_eq!(updated.value, 2);

    let found = repo.read(&id).await.unwrap();
    assert_eq!(found, Some(updated));
  }

  #[tokio::test]
  async fn test_delete() {
    let repo = InMemoryTestRepo::new();
    let created = repo.create(TestData { name: "temp".into(), value: 0 }).await.unwrap();

    let id = rid_key(&created.id);
    repo.delete(&id).await.unwrap();

    let found = repo.read(&id).await.unwrap();
    assert_eq!(found, None);
  }

  #[tokio::test]
  async fn test_crud_lifecycle() {
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
}
