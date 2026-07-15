# Testing

## Test Framework

All tests use `#[tokio::test]` (async runtime). Assertions use standard `assert!`/`assert_eq!`/`assert_ne!`. There are no third-party assertion crates and no `[dev-dependencies]` — all test dependencies are already regular workspace deps.

## Test Module Placement

Tests are defined **inline** at the bottom of the source file they test, gated with `#[cfg(test)]`:

```rust
#[cfg(test)]
mod my_tests {
  // imports, helpers, #[tokio::test] functions
}
```

Module naming convention: `<name>_tests` (e.g., `sync_tests`, `execution_tests`, `local_db_tests`, `remote_db_tests`). Production functions under test are accessed via `super::`.

## Running Tests

```bash
cargo test                    # all workspace tests
cargo test -p remex-core      # core tests only
cargo test -p remex-endpoint  # endpoint tests only
cargo test sync_tests         # tests matching module name
cargo test execution_tests    # execution seam tests
cargo test local_db_tests     # local DB / session tests
cargo test remote_db_tests    # remote DB actor tests
```

## Testing Pattern: Seam Functions + In-Memory Adapter

Database-dependent logic is tested via the **seam function** pattern (see [Testable Database Code with DbOperator](#testable-database-code-with-dboperator)). Each seam function accepts `&dyn DbOperator<Record = X, Input = Y>` as a parameter, and test modules generate an in-memory adapter via `impl_in_memory_db_operator!`.

**Recipe:**

1. Extract the logic into a function accepting `&dyn DbOperator`
2. In the test module, do `impl_in_memory_db_operator!(InMemoryXRepo, RecordType, InputType, "table")`
3. Instantiate `InMemoryXRepo::new()` in the test
4. Call the seam function and assert on the result

**Current endpoint seam tests:**
- `sync_job_to_cache`: 5 tests in `endpoint/src/async_tasks/jobs/sync.rs`
- `should_skip_job`: 4 tests in `endpoint/src/async_tasks/jobs/execution.rs`
- `mark_job_completed`: 4 tests in `endpoint/src/async_tasks/jobs/execution.rs`

`LocalDbActor` tests in `endpoint/src/async_tasks/local_db.rs` cover session load/save and execution cache round-trips.

## Testing Pattern: Match-Based Error Inversion

For integration tests that verify both success and failure paths, use a terminal
`match` statement that documents the expected outcome and panics on unexpected ones.
This ensures **all tests pass** when the system behaves correctly — tests that
expect errors are not written "expecting to fail," they are written to assert
that the correct error occurs.

**Pattern:**

```rust
// Test that expects SUCCESS:
match do_something().await {
  Ok(value) => println!("operation succeeded as expected: {value:?}"),
  Err(e) => panic!("operation should have succeeded: {e}"),
}

// Test that expects FAILURE:
match do_something().await {
  Ok(value) => panic!("operation should have failed, got: {value:?}"),
  Err(e) => println!("operation failed as expected: {e}"),
}
```

**Rules:**
- Every test ends with exactly one `match` statement (or one per logical step)
- The `Ok` or `Err` arm that represents the **expected** outcome calls `println!` to document it
- The arm representing the **unexpected** outcome calls `panic!` to invert the result
- Helper functions (`do_signup`, `do_signin`, etc.) return `Result<_, _>` and **never** call `.unwrap()` or `.expect()` — they propagate errors with `?`

## Testable Database Code with DbOperator

All database operations go through the `DbOperator` trait (`core/src/db/mod.rs`):

```rust
#[async_trait]
pub trait DbOperator: Send + Sync {
  type Record: Send + Sync + 'static;
  type Input: Send + Sync + 'static;

  async fn create(&self, input: Self::Input) -> Result<Self::Record, DbError>;
  async fn read(&self, id: &str) -> Result<Option<Self::Record>, DbError>;
  async fn update(&self, id: &str, input: Self::Input) -> Result<Self::Record, DbError>;
  async fn list(&self) -> Result<Vec<Self::Record>, DbError>;
  async fn delete(&self, id: &str) -> Result<(), DbError>;
}
```

### Seam Pattern

To make a function testable, extract it to accept `&dyn DbOperator` instead of directly querying SurrealDB:

```rust
// ❌ Not testable — tightly coupled to SurrealDB
async fn do_stuff(db: &Surreal<Db>, id: &str) -> Result<(), Error> {
  db.query("SELECT * FROM table WHERE field = $id").bind(("id", id)).await?;
  // ...
}

// ✅ Testable — accepts any adapter via &dyn DbOperator
async fn do_stuff(
  id: &str,
  repo: &dyn DbOperator<Record = MyRecord, Input = MyInput>,
) -> Result<(), Error> {
  let records = repo.list().await?;
  if let Some(record) = records.iter().find(|r| r.field == id) {
    repo.update(&record.id_as_key(), MyInput { ... }).await?;
  }
  Ok(())
}

// Production caller wires in the real adapter:
let repo = SurrealMyRepo { db: get_local_remex().await? };
do_stuff(&id, &repo).await?;
```

### In-Memory Adapter for Tests

Declarative macros in `core/src/db/adapters.rs` generate adapters:

```rust
use remex_core::impl_in_memory_db_operator;
impl_in_memory_db_operator!(InMemoryMyRepo, MyRecord, MyInput, "table_name");
```

**Requirements:**
- `MyRecord: Send + Sync + Clone + 'static`
- `MyInput: Send + Sync + Clone + 'static`
- `impl From<(String, MyInput)> for MyRecord` (for ID generation)

### Recipe: Writing a Testable Seam

1. **Define the seam function** — takes `&dyn DbOperator<Record = X, Input = Y>` as its last parameter
2. **Update the caller** — create the real adapter (`SurrealXRepo { db }`) and pass a reference
3. **Generate an in-memory adapter** — `impl_in_memory_db_operator!` in the test module
4. **Write tests** — instantiate `InMemoryXRepo::new()`, call the seam function, assert on results

### Reference Examples in this Repo

| Seam Function | File | Tests |
|---|---|---|
| `sync_job_to_cache` | `endpoint/src/async_tasks/jobs/sync.rs` | `sync_tests` at the bottom of the file |
| `should_skip_job` | `endpoint/src/async_tasks/jobs/execution.rs` | `execution_tests` at the bottom of the file |
| `mark_job_completed` | `endpoint/src/async_tasks/jobs/execution.rs` | `execution_tests` at the bottom of the file |

### Adapter Generation Macros

Both macros handle `id` generation differently:

- **`impl_surreal_db_operator!(pub Name, Record, Input, "table", "ns", "db")`**: Gets the real DB handle. `create` uses `CREATE table CONTENT $data` (auto-generates ID). `update` uses `UPDATE $id MERGE $data` for partial updates. `list` uses `SELECT * FROM table`.

- **`impl_in_memory_db_operator!(pub Name, Record, Input, "table")`**: `create` generates a UUID v4 string key, calls `From<(String, Input)>`. `update` replaces via HashMap insert (UPSERT semantics). `list` clones all HashMap values.

### Notes

- Use `list()` + iterator filtering for field-based lookups (the trait only supports `read` by record ID)
- `update` has UPSERT semantics in both adapters
- The in-memory adapter is **not persistent** — each test gets a fresh state
- Always clean up unused imports when refactoring (the caller may no longer need `Surreal`, `RecordId`, etc. directly)
