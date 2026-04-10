# AGENTS.md

## Directory Structure

The directory and project are laid out as such:
'/core' - The shared library for all remex executables
'/server' - The server executable and it's related source code
'/endpoint' - The endpoint executable and it's related source code
'/configurator' - The configurator Vue.js web application

## Systems Design

This system utilizes a client-server architecture at it's core.

remex_core is a shared library that holds all shared logic for the server and the endpoint.

remex_server houses the application's central server which maintains connections via an encrypted TCP socket. It connects to the core SurrealDB database. The messages sent over the TCP socket from the server aren't for the purpose of sending database table queries to the endpoints, but instead is a centralized method of pointing the endpoints to the core database in the cloud.

remex_endpoint is the edge client that both connects with the remex_server and connects to the core SurrealDB database in the cloud. In addition to this, it also manages a local database for caching updates to be sent to the core database, and to reference for jobs that may need to be executed offline. It spawns several background tasks that monitor things like whether a job should be run yet, whether to respond to a server message, etc.

remex_configurator is a standalone Vue.js web application for the end user to create new configurations, modify existing ones, and check on the execution status of each job. It connects directly to its own SurrealDB database (separate from the core database used by endpoints).

## CSS/SCSS styles

All styles should use colors from the themes, and not hard-coded colors. If a theme color needs to be added, it should be added to the theme file.

## Database Patterns

This codebase uses SurrealDB as its database. All database operations follow a consistent pattern:

### DbOperator Trait

All database models implement the `DbOperator<T, U>` trait defined in `core/src/db/mod.rs`:

```rust
pub trait DbOperator<T, U>
where
  T: surrealdb::types::SurrealValue,
  U: surrealdb::types::SurrealValue,
{
  fn create(obj: U, db: &Surreal<Db>) -> impl Future<Output = Result<Option<T>, DbError>> + Send;
  fn read(id: String, db: &Surreal<Db>) -> impl Future<Output = Result<Option<T>, DbError>> + Send;
  fn push(&mut self, db: &Surreal<Db>) -> impl Future<Output = Result<(), DbError>> + Send;
  fn pull(&self, db: &Surreal<Db>) -> impl Future<Output = Result<Option<T>, DbError>> + Send;
  fn delete(&self, db: &Surreal<Db>) -> impl Future<Output = Result<(), DbError>> + Send;
}
```

- **create**: Insert a new record
- **read**: Fetch a record by ID
- **push**: Update (upsert) an existing record
- **pull**: Refresh a record from the database
- **delete**: Remove a record

### Migrations

Each model has a `migrate` function that uses SurrealDB queries to define tables, fields, and indexes:

```rust
pub async fn migrate(db: &Surreal<Any>) -> Result<(), DbError> {
  db.query("
    USE NS remex DB remex;
    DEFINE TABLE IF NOT EXISTS client SCHEMAFULL;
    DEFINE FIELD IF NOT EXISTS client_name ON TABLE client TYPE string;
    DEFINE FIELD IF NOT EXISTS secret ON TABLE client TYPE string VALUE crypto::argon2::generate($value);
    ...
  ").await?.check()?;
  Ok(())
}
```

### Access Control

Database access uses BEARER authentication with record-level permissions:

```sql
DEFINE ACCESS IF NOT EXISTS endpoint ON DATABASE TYPE BEARER FOR RECORD DURATION FOR GRANT 1d;
```

### Tables

- **client**: Client devices that connect to the system
- **execution**: Job execution records
- **group**: Client groupings
- **job**: Scheduled jobs to be executed

## Communication Protocol

### Packet System

Messages are fragmented into 128-byte fixed-size packets for transmission over TCP:

- **Packet size**: 128 bytes total
- **Payload**: 126 bytes per packet (2 bytes for packet metadata)
- **Header**: `[packet_number, total_packets]`

### Message Contents Types

Messages are classified by their first character prefix:

- **`0` prefix**: Command - executable instructions
- **`1` prefix**: Secret - sensitive data (credentials, tokens)
- **Other**: Log - general logging information

### Stack Allocation

Uses `heapless::Vec` for fixed-capacity, no-heap allocations to ensure memory safety in constrained environments.

## Key Dependencies

| Dependency | Version | Purpose |
|------------|---------|---------|
| actix | 0.13 | Actor system for async message passing |
| surrealdb | 3 | Database with kv-surrealkv and protocol-ws |
| tokio | 1.38.1 | Async runtime |
| aes-gcm | 0.10.3 | Encryption for TCP socket communication |
| heapless | 0.9.2 | Fixed-capacity collections |
| chrono | 0.4.38 | Date/time handling |
| tracing | 0.1.40 | Structured logging |
| uuid | 1.6 | Unique identifiers (v4, v7) |

## Error Handling

This codebase uses two error handling approaches:

### thiserror

For custom error types with enum variants:

```rust
#[derive(thiserror::Error, Debug)]
pub enum DbError {
  #[error(transparent)]
  SurrealDb(#[from] surrealdb::Error),
  #[error("Operation failed: {0}")]
  OperationFailed(String),
}
```

### anyhow

For general application error handling with context propagation.

When adding new errors, prefer `thiserror` for domain-specific errors and `thiserror::Error` derive macro.
