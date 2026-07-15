# Remex Server — Architecture & Operation

> **Transitional / Legacy Notice**
>
> The `server/` crate is a **transitional migration utility**. It connects to SurrealDB and runs the core migrations, and it still starts a legacy TCP listener on `127.0.0.1:4269` using the `RemexServer`/`RemexSession` actors. Endpoints now authenticate directly to SurrealDB via `endpoint_access` and do **not** connect to this server.

## Purpose

The **Remex Server** crate historically handled endpoint authentication over an encrypted TCP socket. That path has been superseded by direct SurrealDB authentication. Today the server crate is **transitional** and is used primarily to run database migrations. It still starts the legacy TCP listener, but endpoints do not connect to it.

## System Architecture

```
┌─────────────────────────────────────────────────────────┐
│                    Remex Server                         │
│                                                         │
│  ┌──────────────┐    ┌──────────────┐                   │
│  │ RemexServer  │◄──►│ SessionMap   │                   │
│  │ (singleton)  │    │ <client_id,  │                   │
│  │              │    │  Addr>       │                   │
│  └──────┬───────┘    └──────────────┘                   │
│         │                                               │
│    ┌────┴────┐     ┌──────────────┐                     │
│    │ TCP     │     │ RemexSession │                     │
│    │ Listener│────►│ (per-conn)   │                     │
│    │ :4269   │     │ + Heartbeat  │                     │
│    └─────────┘     └──────┬───────┘                     │
│                           │                             │
│                    ┌──────▼───────┐                     │
│                    │ ClientCodec  │                     │
│                    │ AES-256-GCM  │                     │
│                    │ encrypted    │                     │
│                    └──────────────┘                     │
└───────────────────────────┬─────────────────────────────┘
                            │
                            ▼
                   ┌─────────────────┐
                   │  Core SurrealDB │
                   │  (migrations)   │
                   └─────────────────┘
```

### Key Components

| Component | Purpose |
|-----------|---------|
| **RemexServer** | Singleton actor. Central hub managing all connected sessions via `SessionMap`. Handles `ClientConnect` and `ClientDisconnect` messages. |
| **RemexSession** | Per-connection actor. Handles TCP I/O, message decoding/encoding, and heartbeat monitoring. |
| **SessionMap** | Thread-safe `HashMap<client_id, Addr<RemexSession>>`. Enforces one active session per client. |
| **ClientCodec** | Actix codec for AES-256-GCM encrypted, length-prefixed TCP framing. |

## Startup Flow

1. **CLI parsing** — Reads `--debug` and `REMEX_DEBUG` env var
2. **Logging initialization** — DEBUG level if `--debug` or `REMEX_DEBUG` set
3. **SurrealDB connection** — Connects via `DB_ENDPOINT` env var (default: `mem://`), signs in as root
4. **Database migrations** — Runs all core migrations (`client`, `execution`, `group`, `job`, `user`, `refresh_token`, `audit_log`, `config`)
5. **Actor system bootstrap** — Creates `RemexServer` actor with session map and DB connection
6. **TCP listener launch** — Binds to `127.0.0.1:4269`, accepts connections in a loop
7. **Graceful shutdown** — Listens for Ctrl-C, shuts down cleanly

**Runtime**: Single-threaded (`current_thread`) with `LocalSet`. All actors and the TCP listener run on one thread.

## Legacy TCP Listener

The TCP listener and the `RemexServer`/`RemexSession` actors are retained from the previous architecture. They are **not used by current endpoints**. The encrypted packet protocol and actor details remain documented in [`core/CORE_ARCHITECTURE.md`](../core/CORE_ARCHITECTURE.md) for reference.

## Database Interactions

### Connection Setup
- **Singleton**: `LazyLock<Surreal<Any>>` static `REMOTE_DB`
- **Endpoint**: `DB_ENDPOINT` env var (default: `mem://` for in-memory)
- **Auth**: Root signin with `DB_PASSWORD` env var (default: `"remex"`)
- **Context**: Namespace `remex`, Database `remex`

### Migrations (run at startup)
| Model | Tables/Access Created |
|-------|----------------------|
| `Client` | `client` table + `endpoint_access` RECORD access |
| `Execution` | `execution` table + indexes on `job_id`, `client_id` |
| `Group` | `group` table + audit event |
| `Job` | `job` table + computed `execution_status` + audit event |
| `User` | `user` table + `configurator_access` RECORD access |
| `RefreshToken` | `refresh_token` table |
| `AuditLog` | `audit_log` table |
| `Config` | `global_config`, `user_config` tables (ns: remex, db: config) |

> **Note:** No audit event is currently defined for `client`. Audit events exist only for `job` and `group`.

## Key Design Decisions

### Direct Database Access
Current endpoints authenticate directly to SurrealDB via `endpoint_access`. The server crate no longer sits in the authentication path for normal endpoint operation. It remains as a migration utility and legacy TCP listener while the transition completes.

### Single-Threaded Runtime
Uses `#[tokio::main(flavor = "current_thread")]` with `LocalSet`. All actors and the TCP listener run on one thread. This simplifies concurrency (no cross-thread synchronization needed) but limits throughput. Acceptable for the expected scale of endpoint connections.

### Actix Actor System
Used for structured concurrent message passing between the central `RemexServer` and per-connection `RemexSession` actors. Sessions communicate with the server via typed messages (`ClientConnect`, `ClientDisconnect`), enabling clean separation of connection handling from session management.

### No REST API
The server has no web API. The configurator connects directly to SurrealDB.

### Audit Logging via SurrealDB Events
Audit logging uses `DEFINE EVENT` triggers on `job` and `group` that automatically create `audit_log` entries on CREATE/UPDATE/DELETE. A `client` audit event is not currently implemented.
