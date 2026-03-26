# Migration Guide: Diesel → SurrealDB

## Overview

This document outlines all changes needed to migrate from Diesel (PostgreSQL/SQLite) to SurrealDB.

### Connection Info
- **URL**: `ws://192.168.10.87:8090`
- **User**: `root`
- **Password**: `H@ck3r345`
- **Namespace**: `remex`
- **Database**: `remex`

---

## Part 1: Fix the SurrealDB DAL (Critical)

The current DAL files have incorrect type signatures. Fix these files:

### 1.1 Fix Connection Type

**File**: `core/src/db/surreal/connection.rs`

Change:
```rust
// WRONG:
pub type Db = Arc<RwLock<Surreal<Ws>>>;

// CORRECT:
pub type Db = Arc<RwLock<Surreal<surrealdb::engine::remote::ws::Client>>>;
```

### 1.2 Fix DAL Error Handling

**Files to fix**:
- `core/src/db/surreal/dal/client.rs`
- `core/src/db/surreal/dal/job.rs`
- `core/src/db/surreal/dal/execution.rs`
- `core/src/db/surreal/dal/log.rs`
- `core/src/db/surreal/dal/group.rs`

Change all error references:
```rust
// WRONG:
surrealdb::Error::Api(surrealdb::error::Api::NullReturned)

// CORRECT - use this instead:
surrealdb::Error::Api(surrealdb::err::Api::NullReturned)
```

Or simply return a different error:
```rust
// EASIER FIX - just map to a generic error:
created.ok_or_else(|| surrealdb::Error::Api(surrealdb::err::Api::NullReturned))
```

Actually, the simplest fix is to use `expect` or handle Option differently:
```rust
// Simplest fix - unwrap or provide custom error:
let created = db.create(("clients", client.id.clone()))
    .content(client.clone())
    .await?
    .ok_or("Failed to create client")?;
```

---

## Part 2: Core Module Changes

### 2.1 New Exports in `core/src/db.rs`

Replace the entire file with:
```rust
pub mod surreal;

pub use surreal::*;

pub type Db = surreal::Db;

pub async fn connect() -> Result<Db, surrealdb::Error> {
    surreal::connect_default().await
}

pub async fn connect_with_config(config: &surreal::SurrealConfig) -> Result<Db, surrealdb::Error> {
    surreal::connect(config).await
}

pub async fn migrate() -> Result<(), surrealdb::Error> {
    let db = connect().await?;
    surreal::migrate(&db).await
}
```

---

## Part 3: Actor System Changes (`core/src/actors/`)

### 3.1 File: `core/src/actors/server.rs`

**Change line 27**:
```rust
// OLD:
crate::db::migrate(crate::db::ConnectionType::Postgres)

// NEW:
crate::db::migrate().await;
```

### 3.2 File: `core/src/actors/session.rs`

This file needs significant changes. Here's what to do:

**Remove these imports (lines 15-17)**:
```rust
// DELETE:
use diesel::{
    QueryDsl,
    RunQueryDsl,
};
```

**Replace with**:
```rust
use surrealdb::engine::remote::ws::Client;
use crate::db::surreal::{Client as SurrealClient, Db};
```

**Replace all database calls**:

| Old Code | New Code |
|----------|----------|
| `db::establish_connection_postgres()` | `db::connect().await` |
| `db::establish_connection_sqlite()` | `db::connect().await` |
| `dal::SrvDbOperator` | Use DAL methods directly |
| `model::server::jobs::JobSRV` | `crate::db::surreal::models::Job` |
| `schema::server::jobs` | Use table name "jobs" |
| `diesel::insert_into(...)` | `db.create(("table", id)).content(...)` |
| `diesel::update(...)` | `db.update(("table", id)).content(...)` |
| `diesel::delete(...)` | `db.delete(("table", id)).await` |
| `diesel::select(...)` | `db.select(("table", id)).await` |

**Example transformation**:

```rust
// OLD:
let mut conn = db::establish_connection_postgres();
let client = ClientSRV { id: "1".into(), .. };
client.create_srv(&mut conn).unwrap();

// NEW:
let db = db::connect().await.unwrap();
let client = SurrealClient { id: "1".into(), .. };
let dal = crate::db::surreal::dal::ClientDal::new();
dal.create(&*db.read().await, &client).await.unwrap();
```

---

## Part 4: Codec Changes (`core/src/codec.rs`)

### Update imports and types:

```rust
// OLD:
use crate::db::dal::{
    jobs::Job,
    executions::Execution,
    logs::Log,
};

// NEW - use the surreal models:
use crate::db::surreal::models::{
    Job,
    Execution,
    Log,
};
```

---

## Part 5: Server Handler Changes (`server/src/web/handlers/`)

### 5.1 Pattern for each handler

**Before**:
```rust
fn get_clients() -> HttpResponse {
    let mut pool = remex_core::db::establish_connection_postgres();
    let clients = remex_core::db::dal::clients::Client::read_srv(&client, &mut pool).unwrap();
    HttpResponse::Ok().json(clients)
}
```

**After**:
```rust
async fn get_clients(db: web::Data<remex_core::Db>) -> HttpResponse {
    let dal = remex_core::db::surreal::dal::ClientDal::new();
    match dal.list(&*db.read().await).await {
        Ok(clients) => HttpResponse::Ok().json(clients),
        Err(e) => HttpResponse::InternalServerError().json(e.to_string()),
    }
}
```

### 5.2 Files to update:

| File | # of Changes |
|------|--------------|
| `server/src/web/handlers/clients.rs` | 3 |
| `server/src/web/handlers/jobs.rs` | 10 |
| `server/src/web/handlers/groups.rs` | 8 |

### 5.3 Common changes in handlers:

1. **Add Db to handler signature**:
   ```rust
   async fn handler_name(db: web::Data<remex_core::Db>, ...) -> HttpResponse
   ```

2. **Remove connection establishment**:
   ```rust
   // DELETE: let mut pool = remex_core::db::establish_connection_postgres();
   ```

3. **Use DAL methods**:
   ```rust
   // Instead of: Client { id: "1" }.read_srv(&mut pool)
   // Use:
   let dal = ClientDal::new();
   dal.read(&*db.read().await, "1").await
   ```

4. **Return HttpResponse properly**:
   ```rust
   match result {
       Ok(data) => HttpResponse::Ok().json(data),
       Err(e) => HttpResponse::InternalServerError().body(e.to_string()),
   }
   ```

---

## Part 6: Endpoint Changes (`endpoint/src/main.rs`)

### File: `endpoint/src/main.rs`

**Changes needed (2 locations)**:

1. **Remove SQLite connection**:
   ```rust
   // DELETE:
   let mut dbconn = remex_core::db::establish_connection_sqlite();
   ```

2. **Add SurrealDB connection**:
   ```rust
   // ADD at startup:
   let db = remex_core::db::connect().await.expect("Failed to connect to SurrealDB");
   remex_core::db::migrate().await.expect("Failed to migrate");
   ```

3. **Pass db to state**:
   ```rust
   // Pass to actix web state
   web::Data::new(db)
   ```

---

## Part 7: Web Server Changes (`server/src/web/mod.rs`)

### Add Db to app state:

```rust
HttpServer::new(move || {
    let db = /* get db connection */;
    
    App::new()
        .app_data(web::Data::new(db))
        // ... rest of handlers
})
```

---

## Summary of File Changes

### Create/Overwrite (Done)
- ✅ `core/src/db/surreal/mod.rs`
- ✅ `core/src/db/surreal/connection.rs`
- ✅ `core/src/db/surreal/schema.rs`
- ✅ `core/src/db/surreal/models/*.rs` (7 files)
- ✅ `core/src/db/surreal/dal/*.rs` (5 files)
- ✅ `core/src/db.rs`

### Fix Type Errors
- `core/src/db/surreal/connection.rs` - Change `Ws` to `Client`
- `core/src/db/surreal/dal/*.rs` - Fix error handling

### Update to use SurrealDB
- `core/src/actors/server.rs` - Change migrate call
- `core/src/actors/session.rs` - Rewrite DB operations
- `core/src/codec.rs` - Update imports
- `server/src/web/handlers/clients.rs` - 3 handlers
- `server/src/web/handlers/jobs.rs` - 10 handlers
- `server/src/web/handlers/groups.rs` - 8 handlers
- `server/src/web/mod.rs` - Add state management
- `endpoint/src/main.rs` - 2 locations
- `server/src/main.rs` - May need updates

---

## Quick Test Checklist

After making changes, verify:

1. [ ] `cargo check --package remex-core` - No errors
2. [ ] `cargo check --package remex-server` - No errors
3. [ ] `cargo check --package remex-endpoint` - No errors
4. [ ] SurrealDB server is running at `ws://192.168.10.87:8090`
5. [ ] Database schema initializes on first connect
6. [ ] Server can connect and perform CRUD operations
7. [ ] Endpoint can connect and perform CRUD operations

---

## Troubleshooting

### "Connection type not implemented"
- Ensure you're using `surrealdb::engine::remote::ws::Client` not `Ws`

### "Module not found"
- Make sure `pub mod surreal;` is in `core/src/db.rs`

### Handler type errors
- Handlers must be `async fn` and accept `web::Data<Db>`
- Return type should be `impl Future<Output = HttpResponse>`

### Authentication errors
- Check credentials: user=`root`, pass=`H@ck3r345`
- Ensure SurrealDB has auth enabled
