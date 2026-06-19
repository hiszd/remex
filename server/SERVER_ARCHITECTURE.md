# Remex Server — Architecture & Operation

## Purpose

The **Remex Server** is a centralized TCP relay that handles endpoint enrollment and authentication. It does NOT proxy database queries — its sole purpose is to authenticate edge clients (endpoints) and provision them with SurrealDB bearer tokens so they can connect directly to the core database.

The server serves as the trust anchor of the system:
- Validates endpoint identity using shared secrets and hardware fingerprints
- Provisions per-client credentials and database access tokens
- Tracks connected sessions and enforces single-connection-per-client
- Maintains heartbeat monitoring to detect stale connections

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
                   │  (client table, │
                   │   bearer tokens)│
                   └─────────────────┘
```

### Key Components

| Component | Purpose |
|-----------|---------|
| **RemexServer** | Singleton actor. Central hub managing all connected sessions via `SessionMap`. Handles `ClientConnect` and `ClientDisconnect` messages. |
| **RemexSession** | Per-connection actor. Handles TCP I/O, message decoding/encoding, authentication, and heartbeat monitoring. |
| **SessionMap** | Thread-safe `HashMap<client_id, Addr<RemexSession>>`. Enforces one active session per client. |
| **ClientCodec** | Actix codec for AES-256-GCM encrypted, length-prefixed TCP framing. |
| **Secret Store** | Filesystem-based secret at `~/.config/remex/serversecret`. Shared secret for endpoint signup verification. |

## Startup Flow

1. **CLI parsing** — Reads `--debug`, `--server`, `--port`, `REMEX_SECRET` env var
2. **Logging initialization** — DEBUG level if `--debug` or `REMEX_DEBUG` set
3. **Secret management** — Reads or generates 64-char server secret at `~/.config/remex/serversecret`
4. **SurrealDB connection** — Connects via `DB_ENDPOINT` env var (default: `mem://`), signs in as root
5. **Database migrations** — Runs all core migrations (client, execution, group, job, user, refresh_token, audit_log, config)
6. **Actor system bootstrap** — Creates `RemexServer` actor with session map and DB connection
7. **TCP server launch** — Binds to `127.0.0.1:4269`, accepts connections in a loop
8. **Graceful shutdown** — Listens for Ctrl-C, shuts down cleanly

**Runtime**: Single-threaded (`current_thread`) with `LocalSet`. All actors and the TCP server run on one thread.

## TCP Connection Architecture

### Actor Hierarchy

```
RemexServer (singleton)
  │
  ├── SessionMap<String>  -- HashMap<client_id, Addr<RemexSession>>
  │
  └── For each TCP connection:
        RemexSession (per-connection)
          ├── FramedRead<ClientCodec>  -- decodes incoming ClientRequest
          ├── FramedWrite<ClientCodec> -- encodes outgoing ServerResponse
          ├── Heartbeat timer (1s interval, 15s timeout)
          └── Reference to RemexServer Addr
```

### Connection Lifecycle

```
1. TCP connection accepted
   ↓
2. RemexSession actor created (unauthenticated, unidentified)
   ↓
3. Heartbeat started — sends Ping every 1s, disconnects if no response in 15s
   ↓
4. Client sends SignupClient or SigninClient
   ↓
5. Server validates credentials against SurrealDB
   ↓
6. On success:
   - Sets client_id and authenticated = true
   - Sends ClientConnect to RemexServer
   - RemexServer inserts into SessionMap (rejects duplicates)
   - Returns SignedUp/SignedIn with bearer token + DB URL
   ↓
7. Endpoint uses bearer token to connect directly to SurrealDB
   (no further server involvement for DB operations)
   ↓
8. Ongoing: Ping/Pong heartbeat every 1s
   ↓
9. On disconnect:
   - RemexSession::stopping() sends ClientDisconnect to RemexServer
   - RemexServer removes from SessionMap
```

## Authentication Flow

### Signup (First-Time Enrollment)

```
Endpoint                              Server
   │                                     │
   │── SignupClient(server_secret, ─────►│
   │   client_name, hardware_hash)       │
   │                                     │
   │   Server verifies:                  │
   │   1. server_secret matches stored   │
   │   2. UPSERT client in SurrealDB     │
   │      (argon2-hashed secret)         │
   │   3. ACCESS endpoint GRANT          │
   │      (creates 1-day bearer token)   │
   │                                     │
   │◄── SignedUp(client_id, token, ──────│
   │    client_secret, db_url)           │
   │                                     │
```

**Steps:**
1. Validates `server_secret` against the server's stored secret
2. Generates a new 64-char `client_secret` for this endpoint
3. UPSERTs a `Client` record in SurrealDB with `client_name`, `hardware_hash`, and argon2-hashed `secret`
4. Calls `ACCESS endpoint GRANT FOR RECORD {id}` to generate a bearer token
5. Returns `SignedUp` with client ID, bearer token, client secret, and DB URL
6. Sets `authenticated = true`

### Signin (Returning Connections)

```
Endpoint                              Server
   │                                     │
   │── SigninClient(client_secret, ─────►│
   │   client_name, client_id,           │
   │   hardware_hash)                    │
   │                                     │
   │   Server verifies:                  │
   │   1. Client exists in SurrealDB     │
   │   2. argon2::compare(secret, sent)  │
   │   3. hardware_hash matches          │
   │   4. UPSERT client (refresh)        │
   │   5. ACCESS endpoint GRANT          │
   │      (new bearer token)             │
   │                                     │
   │◄── SignedIn(token, None, db_url) ───│
   │                                     │
```

**Steps:**
1. Queries: `SELECT * FROM client WHERE id = $id AND crypto::argon2::compare(secret, $secret) AND hardware_hash = $hardware_hash`
2. If found, UPSERTs the client record (refreshes `updated_at`)
3. Generates a new bearer token via `ACCESS endpoint GRANT`
4. Returns `SignedIn` with bearer token and DB URL (no new client secret)
5. Sets `authenticated = true`

### Duplicate Connection Prevention

Only one active session per client ID is allowed. When a `ClientConnect` message arrives:
```rust
if self.sessions.exists(&msg.client_id) {
  return Err(DisconnectReason::DuplicateClient);
}
```
The new connection is rejected with `DuplicateClient`, and the existing session remains active.

## Message Protocol

### Wire Format

```
[2 bytes: big-endian u16 payload length] [N bytes: AES-256-GCM encrypted JSON]
```

- **Encryption**: AES-256-GCM with 12-byte nonce prepended to ciphertext
- **Key**: 32-byte constant (FIXME: should not be hardcoded)
- **Nonce**: Generated fresh per message via `OsRng`

### Message Types

**ClientRequest** (endpoint → server):
| Variant | Fields | Purpose |
|---------|--------|---------|
| `SignupClient` | `(server_secret, client_name, hardware_hash)` | First-time enrollment |
| `SigninClient` | `(client_secret, client_name, client_id, hardware_hash)` | Returning authentication |
| `Ping` | — | Heartbeat response |

**ServerResponse** (server → endpoint):
| Variant | Fields | Purpose |
|---------|--------|---------|
| `SignedIn` | `(BearerGrantResponse, Option<String>, String)` | Auth success: `(token, new_secret?, db_url)` |
| `SignedUp` | `(RecordId, BearerGrantResponse, String, String)` | Signup success: `(client_id, token, client_secret, db_url)` |
| `Disconnect` | `DisconnectReason` | Connection termination |
| `Ping` | — | Heartbeat probe |

**DisconnectReason**:
| Variant | Trigger |
|---------|---------|
| `AuthFailed` | Invalid server secret or client credentials |
| `InvalidClientId` | Client ID not found in database |
| `DuplicateClient` | Another session already active for this client |
| `HeartbeatFailed` | No ping response within 15 seconds |
| `Unknown(String)` | Other errors |

## Heartbeat Monitoring

- **Interval**: Server sends `Ping` every 1 second
- **Timeout**: If 15 seconds pass without a client `Ping` response, connection is terminated
- **Mechanism**: `actix::prelude::ActorContext::run_interval(1s)` sends `Ping`; each received `Ping` from client updates `act.hb = Instant::now()`
- **Check**: On each tick, compares `Instant::now() - act.hb` against 15-second threshold

## Database Interactions

### Connection Setup
- **Singleton**: `LazyLock<Surreal<Any>>` static `REMOTE_DB`
- **Endpoint**: `DB_ENDPOINT` env var (default: `mem://` for in-memory)
- **Auth**: Root signin with `DB_PASSWORD` env var (default: `"remex"`)
- **Context**: Namespace `remex`, Database `remex`

### Migrations (run at startup)
| Model | Tables Created |
|-------|---------------|
| `Client` | `client` table + `endpoint` BEARER access + audit event |
| `Execution` | `execution` table + indexes on `job_id`, `client_id` |
| `Group` | `group` table + audit event |
| `Job` | `job` table + computed `execution_status` + audit event |
| `User` | `user` table + `configurator_access` RECORD access |
| `RefreshToken` | `refresh_token` table |
| `AuditLog` | `audit_log` table |
| `Config` | `global_config`, `user_config` tables (ns: remex, db: config) |

### Bearer Token Generation
```rust
pub async fn get_endpoint_bearer_token(id: RecordId, db: &Surreal<Any>) -> Result<Option<BearerGrantResponse>> {
  db.query(format!("ACCESS endpoint GRANT FOR RECORD {};", id.to_sql())).await?
}
```
Uses SurrealDB's `DEFINE ACCESS endpoint ON DATABASE TYPE BEARER FOR RECORD DURATION FOR GRANT 1d` to generate a 1-day bearer token scoped to the specific client record.

## Secret Management

### Server Secret
- **Location**: `~/.config/remex/serversecret`
- **Format**: 64-character alphanumeric string
- **Purpose**: Shared secret for endpoint signup verification
- **Lifecycle**: Generated on first startup if missing, persisted as plain text

### Client Secret
- **Storage**: SurrealDB `client` table, `secret` field
- **Hashing**: `crypto::argon2::generate($value)` on field definition
- **Purpose**: Per-client credential for signin authentication
- **Lifecycle**: Generated during signup, returned once to the endpoint, never returned again

### Encryption Key
- **Location**: Hardcoded 32-byte constant in `core/src/codec.rs`
- **Algorithm**: AES-256-GCM
- **FIXME**: Noted in code that this should not be hardcoded

## Key Design Decisions

### Direct Database Access
The server does NOT proxy database queries. After authentication, endpoints receive a SurrealDB bearer token and DB URL, then connect directly to the database. This eliminates the server as a bottleneck and allows endpoints to operate independently once authenticated.

### Single-Threaded Runtime
Uses `#[tokio::main(flavor = "current_thread")]` with `LocalSet`. All actors and the TCP server run on one thread. This simplifies concurrency (no cross-thread synchronization needed) but limits throughput. Acceptable for the expected scale of endpoint connections.

### Actix Actor System
Used for structured concurrent message passing between the central `RemexServer` and per-connection `RemexSession` actors. Sessions communicate with the server via typed messages (`ClientConnect`, `ClientDisconnect`), enabling clean separation of connection handling from session management.

### No REST API
The server has no web API. The configurator connects directly to SurrealDB. The server's sole purpose is TCP-based endpoint authentication and token provisioning.

### Hardcoded DB URL
The database URL `"192.168.10.87:8090"` is hardcoded in the `SignedUp`/`SignedIn` response handlers rather than being dynamically configured from the server's own DB connection. This should be made configurable.

### Audit Logging via SurrealDB Events
The client table migration includes `DEFINE EVENT audit_client` which automatically creates `audit_log` entries on CREATE/UPDATE/DELETE, providing an automatic audit trail without application-level code.
