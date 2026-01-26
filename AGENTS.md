# Remex Development Guide for Agents

This document provides essential information for AI agents working on the Remex project. It serves as the primary source of truth for build procedures, code style, and architectural patterns.

## 1. Project Overview & Architecture

Remex is a distributed system built in Rust. It follows a workspace architecture with three main crates:

- **`core`**: Contains shared business logic, database models, actor definitions, and utility functions. Most of the development happens here.
- **`server`**: The central backend application. It manages the primary Postgres database and orchestrates jobs.
- **`endpoint`**: The client-side agent application (CLI/Daemon). It uses a local SQLite database.

### Key Technologies
- **Runtime**: `tokio` (Async/Await).
- **Actors**: `actix` framework for message-passing concurrency.
- **Database**:
  - `sqlx` for database connection, execution, and migrations.
  - `sea-query` for dynamic query construction (avoids raw SQL strings).
  - `Postgres` (Server) and `SQLite` (Endpoint).
- **Logging**: `tracing` for structured logging.

## 2. Build, Test, and Run Commands

### Basic Operations
- **Build Project**: `cargo build`
- **Build Release**: `cargo build --release`
- **Check (Fast)**: `cargo check`
- **Format Code**: `cargo fmt` (Strictly enforced via `rustfmt.toml`)
- **Lint Code**: `cargo clippy`

### Testing
Tests are crucial. Always run tests after modifying logic.
- **Run All Tests**: `cargo test`
- **Run Specific Package**: `cargo test -p remex-core`
- **Run Specific Test**: `cargo test test_name_substring`
- **Run Specific Module**: `cargo test modules::test_module`

### Running Applications
- **Run Server**: `cargo run -p remex-server`
- **Run Endpoint**: `cargo run -p remex-endpoint`

### Database Operations
- **Run Migrations (Server)**:
  `sqlx migrate run --source migrations/server --database-url <POSTGRES_URL>`
- **Run Migrations (Endpoint)**:
  `sqlx migrate run --source migrations/endpoint --database-url <SQLITE_URL>`
- **Create Migration**:
  `sqlx migrate add -r <name>` (creates reversible migration)

## 3. Code Style & Conventions

### Formatting & Layout
- **Indentation**: 2 spaces (Enforced).
- **Line Length**: 100 characters.
- **Imports**: Grouped by `StdExternalCrate`.
  ```rust
  use std::collections::HashMap; // Std
  use actix::prelude::*;         // External
  use crate::db::models::Job;    // Internal
  ```
- **Ordering**: Imports are automatically reordered by `cargo fmt`.

### Naming Conventions
- **Files/Modules**: `snake_case` (e.g., `job_scheduler.rs`, `db_utils.rs`).
- **Structs/Enums**: `CamelCase` (e.g., `JobStatus`, `ClientConnection`).
- **Functions/Variables**: `snake_case` (e.g., `fetch_active_jobs`, `user_id`).
- **Constants/Statics**: `SCREAMING_SNAKE_CASE` (e.g., `MAX_RETRIES`, `DEFAULT_TIMEOUT`).
- **Database Tables**: `snake_case`, pluralized (e.g., `jobs`, `clients`, `job_logs`).

### Error Handling
- **Application Logic**: Use `anyhow::Result` for handlers and top-level logic where exact error types matter less.
- **Library/Core**: Use `thiserror` for defining custom error enums if the caller needs to handle specific variants.
- **Database**: `sqlx::Error` is common. Map it to domain errors if necessary.
- **Logging**: Use `tracing::error!(%e, "message")` to log errors with context. Avoid `println!` or `eprintln!`.

### Database Patterns
- **Query Building**: ALWAYS use `sea-query` to construct complex queries. Do not write raw SQL strings unless absolutely necessary (and then use `sqlx::query!`).
- **Sea-Query Style**:
  - Use `Expr::val(value)` for parameters.
  - Use `Alias::new("name")` for table/column names (unless `Iden` enums are already defined).
  - **Json**: When using Postgres JSON functions, use `Func::cust` or `Expr::cust` carefully.
- **Connection**: Pass `&sqlx::Pool<Postgres>` or `&sqlx::Pool<Sqlite>` explicitly, or use the `Pools` enum wrapper.

### Actor Pattern
- **Messages**: Define messages as structs deriving `Message`.
  ```rust
  #[derive(Message)]
  #[rtype(result = "Result<(), anyhow::Error>")]
  pub struct MyMessage { ... }
  ```
- **Handlers**: Implement `Handler<MyMessage>` for your Actor struct.
- **Context**: Use `Context<Self>` or `AsyncContext<Self>` for accessing actor state/lifecycle.

## 4. Development Workflow for Agents

1.  **Analyze**: Before writing code, use `ls`, `grep`, and `read` to understand the existing context. Do not guess types or function signatures.
2.  **Plan**: Formulate a plan. If a database change is needed, plan the migration first.
3.  **Implement**:
    - Add/Edit code using `write` or `edit`.
    - Follow the naming and style conventions strictly.
    - If adding a new module, ensure it is registered in `mod.rs` or `lib.rs`.
4.  **Verify**:
    - Run `cargo check` to catch compilation errors early.
    - Run `cargo test` to ensure no regressions.
    - Fix any `clippy` warnings.

## 5. Troubleshooting Common Issues

- **Sea-Query Type Mismatches**:
  - `Expr::col` returns a `SimpleExpr`.
  - `Expr::val` returns a `SimpleExpr`.
  - `Func::cust` returns a `FunctionCall`.
  - When putting these into a `Vec` (e.g., for `.args()`), they must be the same type.
  - **Solution**: Use chained `.arg()` calls instead of `.args([...])` to avoid array homogeneity issues in Rust.
  - **Example**: `Func::cust(...).arg(Expr::val("a")).arg(Expr::col(...))`

- **Async Block in Actix Handler**:
  - Actix handlers are synchronous by default. To do async work (like DB calls), use `Box::pin(async move { ... })` returning a `ResponseFuture` or `AtomicFuture`, or use `Actor::Context` with `ctx.spawn(actix::fut::wrap_future(...))`.

- **Diesel vs SQLx**:
  - This project uses **SQLx**, not Diesel. Do not attempt to use Diesel commands or patterns.

## 6. Directory Structure
- `core/src/db/model`: Database structs (Serde + FromRow).
- `core/src/db/actions`: Logic for DB operations (CRUD).
- `core/src/actors`: Actix actor definitions.
- `server/src`: Server entry point and HTTP/Socket configuration.
- `migrations`: SQL migration files (Server/Postgres and Endpoint/SQLite).
