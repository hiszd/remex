# Remex Core — Architecture & Operation

## Purpose

**Remex Core** is the shared library that holds all common logic for both the Remex Server and Remex Endpoint. It provides the communication protocol, encryption, actor system, database abstractions, and all domain data models. By centralizing these concerns, both binaries use identical types, serialization, and database logic without duplication.

## Module Organization

```
core/src/
├── lib.rs              # Public exports: Packet, Message, MessageContents
├── codec.rs            # AES-256-GCM encryption, TCP framing, ClientRequest/ServerResponse
├── sessionmap.rs       # Thread-safe session registry (HashMap wrapper)
├── db/
│   ├── mod.rs          # DbOperator trait, migrate(), BearerGrantResponse
│   ├── connection.rs   # DbClients struct (local + remote DB holder)
│   └── model/
│       ├── clients.rs      # Client model + endpoint BEARER access
│       ├── executions.rs   # Execution model + indexes
│       ├── groups.rs       # Group model + audit event
│       ├── jobs.rs         # Job model + computed execution_status
│       ├── users.rs        # User model + configurator_access
│       ├── refresh_tokens.rs # RefreshToken model
│       ├── audit.rs        # AuditLog model (append-only)
│       └── config.rs       # Config tables (separate database)
└── actors/
    ├── server.rs       # RemexServer actor (singleton hub)
    ├── server/msg.rs   # ClientConnect, ClientDisconnect handlers
    ├── session.rs      # RemexSession actor (per-connection TCP handler)
    └── session/msg.rs  # SignupClient, SigninClient, Disconnect handlers
```

## Communication Protocol

### Packet System

The wire protocol uses fixed-size 128-byte packets for transmission over TCP:

```
┌─────────────────────────────────────────────────────────┐
│                    RawPacket (128 bytes)                 │
├──────────────┬──────────────────────────────────────────┤
│ Packet       │ Packet Payload (126 bytes)               │
│ Header       │                                          │
│ (2 bytes)    │                                          │
├──────┬───────┼──────────────────────────────────────────┤
│number│ total │  information: StackVec<u8, 126>          │
│ (u8) │ (u8)  │                                          │
└──────┴───────┴──────────────────────────────────────────┘
```

| Field | Type | Description |
|-------|------|-------------|
| `number` | `u8` | 1-indexed packet number in the sequence |
| `total` | `u8` | Total number of packets in the message |
| `information` | `StackVec<u8, 126>` | Payload chunk (max 126 bytes) |

**Key design choices:**
- **`heapless::Vec`** (`StackVec`) for stack-allocated, no-heap collections — critical for memory-constrained endpoint environments
- **2-byte header** leaves 126 bytes per packet for payload
- **`RawPacket = [u8; 128]`** — the actual wire-level unit

### Message Fragmentation

```rust
pub struct Message {
  msg: String,
  packets: Vec<Packet>,
}
```

Messages are strings that are automatically fragmented into packets:

- **Fragmentation** (`packets_from_string`): Calculates total packets as `(msg.len() / 126) + 1`, slices the string bytes into 126-byte chunks, each becoming a numbered `Packet`
- **Reassembly** (`string_from_packets`): Iterates packets in order, converts non-zero bytes back to chars, concatenates
- **`update()`**: Replaces the message string and re-fragments
- **`From<Vec<Packet>>`**: Reconstructs a `Message` from received packets

### Message Classification

```rust
pub enum MessageContents {
  Command(String),  // Messages starting with "0"
  Secret(String),   // Messages starting with "1"
  Log(String),      // Everything else
}
```

Messages are classified by their first character prefix. `From<Message>` automatically classifies based on the string's first character.

### Codec (Encryption + Framing)

**Wire format:**
```
[2 bytes: big-endian u16 payload length] [N bytes: AES-256-GCM encrypted JSON]
```

| Component | Detail |
|-----------|--------|
| **Encryption** | AES-256-GCM via `aes_gcm` crate |
| **Key** | 32-byte constant (FIXME: should not be hardcoded) |
| **Nonce** | 12 bytes, generated fresh per message via `OsRng`, prepended to ciphertext |
| **Framing** | 2-byte big-endian length prefix before each encrypted payload |
| **Serialization** | JSON via `serde_json` |

**Two codec types (mirror images):**

| Codec | Decodes | Encodes | Used By |
|-------|---------|---------|---------|
| `ClientCodec` | `ClientRequest` | `ServerResponse` | Server |
| `ServerCodec` | `ServerResponse` | `ClientRequest` | Endpoint |

### Protocol Messages

**ClientRequest** (endpoint → server):
```rust
#[serde(tag = "cmd", content = "data")]
pub enum ClientRequest {
  SignupClient(String, String, String),     // (server_secret, client_name, hardware_hash)
  SigninClient(String, String, RecordId, String), // (client_secret, client_name, client_id, hardware_hash)
  Ping,
}
```

**ServerResponse** (server → endpoint):
```rust
#[serde(tag = "cmd", content = "data")]
pub enum ServerResponse {
  SignedIn(BearerGrantResponse, Option<String>, String),  // (token, new_secret?, db_url)
  SignedUp(RecordId, BearerGrantResponse, String, String), // (client_id, token, client_secret, db_url)
  Disconnect(DisconnectReason),
  Ping,
}
```

**DisconnectReason:**
| Variant | Trigger |
|---------|---------|
| `AuthFailed` | Invalid server secret or client credentials |
| `InvalidClientId` | Client ID not found in database |
| `DuplicateClient` | Another session already active for this client |
| `HeartbeatFailed` | No ping response within 15 seconds |
| `Unknown(String)` | Other errors |

## Actor System

### Hierarchy

```
RemexServer (singleton actor)
  │
  ├── SessionMap<String>  -- HashMap<client_id, Addr<RemexSession>>
  │
  └── For each TCP connection:
        RemexSession (per-connection actor)
          ├── FramedRead<ClientCodec>  -- decodes incoming ClientRequest
          ├── FramedWrite<ClientCodec> -- encodes outgoing ServerResponse
          ├── Heartbeat timer (1s interval, 15s timeout)
          └── Reference to RemexServer Addr
```

### RemexServer

```rust
pub struct RemexServer {
  pub sessions: SessionMap<String>,           // Tracks all connected sessions by client_id
  pub migrated: bool,                         // Whether DB migration has run
  pub secret: Option<String>,                 // Server secret for endpoint signup
  pub db: Option<Surreal<Any>>,               // Database connection
  pub client_sessions: Arc<Mutex<HashMap<String, ClientSessionInfo>>>,
}
```

**Responsibilities:**
- Central hub for all connected sessions
- Enforces single-connection-per-client via `SessionMap`
- Handles `ClientConnect` (insert, reject duplicates) and `ClientDisconnect` (remove, log)

### RemexSession

```rust
pub struct RemexSession {
  id: String,                    // UUID v4 session ID
  client_id: Option<RecordId>,   // SurrealDB record ID after auth
  name: Option<String>,          // Client name
  server_secret: String,         // Server secret for signup verification
  authenticated: bool,           // Auth state
  identified: bool,              // Identity verified state
  addr: Addr<RemexServer>,       // Reference to parent server actor
  db: Option<Surreal<Any>>,      // DB connection for auth queries
  hb: Instant,                   // Last heartbeat timestamp
  framed: FramedWrite<...>,      // Encrypted TCP write half
}
```

**Responsibilities:**
- TCP I/O via `StreamHandler` (receives decoded `ClientRequest`)
- Authentication via `SignupClient` / `SigninClient` message handlers
- Heartbeat monitoring: sends `Ping` every 1s, disconnects if no response in 15s
- Notifies `RemexServer` of disconnect via `ClientDisconnect` on session end

### SessionMap

```rust
pub struct SessionMap<T> {
  pub sessions: HashMap<T, actix::Addr<RemexSession>>,
}
```

Generic over key type `T` (`Eq + Hash + Clone + Display`). Operations:
- **`insert(id, addr)`** — Fails if key already exists (duplicate prevention)
- **`remove(&id)`** — Removes and returns the session address
- **`exists(&id)`** — Key presence check
- **`change_id(old, new)`** — Atomically re-keys a session

### TCP Server Entry Point

```rust
pub async fn tcp_server(s: &str, secret: &str, server: Addr<RemexServer>, db: Option<Surreal<Any>>)
```

Binds a `TcpListener` on the given address. For each incoming connection: splits stream into read/write halves, creates a `RemexSession` actor with `FramedRead`/`FramedWrite` using `ClientCodec`.

## Database Abstractions

### DbOperator Trait

```rust
pub trait DbOperator<T, U>
where T: SurrealValue, U: SurrealValue
{
  fn create(obj: U, db: &Surreal<Db>) -> impl Future<Output = Result<Option<T>, DbError>> + Send;
  fn read(id: String, db: &Surreal<Db>) -> impl Future<Output = Result<Option<T>, DbError>> + Send;
  fn push(&mut self, db: &Surreal<Db>) -> impl Future<Output = Result<(), DbError>> + Send;
  fn pull(&self, db: &Surreal<Db>) -> impl Future<Output = Result<Option<T>, DbError>> + Send;
  fn delete(&self, db: &Surreal<Db>) -> impl Future<Output = Result<(), DbError>> + Send;
}
```

| Method | Purpose |
|--------|---------|
| `create` | Insert new record, return the created entity |
| `read` | Fetch by ID string |
| `push` | Upsert (update existing record, mutates self with returned data) |
| `pull` | Refresh self from database |
| `delete` | Remove record by ID |

**Type parameters:**
- **T** = Return type (full record with ID)
- **U** = Input type (data without ID, used for creation)

### Migration Orchestration

```rust
pub async fn migrate(db: &Surreal<Any>) -> Result<(), DbError>
```

Runs migrations in order:
1. `Client::migrate` → `remex` DB
2. `Execution::migrate` → `remex` DB
3. `Group::migrate` → `remex` DB
4. `Job::migrate` → `remex` DB
5. `User::migrate` → `remex` DB
6. `RefreshToken::migrate` → `remex` DB
7. `AuditLog::migrate` → `remex` DB
8. `Config::migrate` → `config` DB (separate database)

### DbClients

```rust
pub struct DbClients {
  pub local: Surreal<Db>,              // Local SurrealKV (endpoint offline cache)
  pub remote: Option<Surreal<Client>>, // Remote WebSocket to core DB
}
```

Used by the endpoint to manage both local and remote database connections through the same `DbOperator` interface.

### Bearer Token Generation

```rust
pub async fn get_endpoint_bearer_token(id: RecordId, db: &Surreal<Any>)
  -> Result<Option<BearerGrantResponse>, DbError>
```

Executes `ACCESS endpoint GRANT FOR RECORD <id>` to generate a 1-day BEARER token scoped to the specific client record.

## Database Models

### Client

**Table**: `client` (SCHEMAFULL) in `remex` DB

| Field | Type | Notes |
|-------|------|-------|
| `client_name` | `string` | |
| `secret` | `string` | Auto argon2-hashed via `VALUE crypto::argon2::generate($value)` |
| `hardware_hash` | `string` | UNIQUE index |
| `last_seen` | `option<datetime>` | Connection tracking |
| `connection_history` | `array<object>` | Default `[]` |
| `created_at` | `datetime` | READONLY, default `now()` |
| `updated_at` | `datetime` | READONLY, value `now()` |

**Access**: `DEFINE ACCESS endpoint ON DATABASE TYPE BEARER FOR RECORD DURATION FOR GRANT 1d`

**Audit**: `DEFINE EVENT audit_client` fires on CREATE/UPDATE/DELETE

---

### Job

**Table**: `job` (SCHEMAFULL) in `remex` DB

| Field | Type | Notes |
|-------|------|-------|
| `job_name` | `string` | |
| `job_shell` | `string` | Shell executable path |
| `job_command` | `string` | Command to execute |
| `job_type` | `object FLEXIBLE` | `{ Instant: {} }`, `{ Scheduled: datetime }`, `{ Recurring: [datetime, duration] }` |
| `execution_status` | `object COMPUTED` | Derived from execution records (see below) |
| `enabled` | `object FLEXIBLE` | `{ Draft: {} }`, `{ Enabled: {} }`, `{ Disabled: {} }` |
| `assignments` | `array<record<client | group>>` | Default `[]` |
| `timeout` | `option<duration>` | Execution timeout |
| `created_at` | `datetime` | READONLY |
| `updated_at` | `datetime` | READONLY |

**Computed `execution_status` logic:**
1. No executions → `{ Pending: {} }`
2. Any Failed → `{ Failed: {} }`
3. ALL TimedOut → `{ TimedOut: {} }`
4. ALL Completed → `{ Completed: {} }`
5. Otherwise → `{ Running: {} }`

**Audit**: `DEFINE EVENT audit_job` fires on CREATE/UPDATE/DELETE

---

### Execution

**Table**: `execution` (SCHEMAFULL) in `remex` DB

| Field | Type | Notes |
|-------|------|-------|
| `job_id` | `record<job>` | Indexed (`idx_job_id`) |
| `client_id` | `record<client>` | Indexed (`idx_client_id`) |
| `status` | `object FLEXIBLE` | `Running`, `Completed`, `Failed`, `Cancelled`, `TimedOut` |
| `output` | `string` | Command stdout + stderr |
| `command` | `string` | Executed command |
| `exit_code` | `string` | |
| `execution_start` | `datetime` | |
| `execution_end` | `datetime` | |
| `created_at` | `datetime` | READONLY |
| `updated_at` | `datetime` | READONLY |

**Permissions**: `select FULL`, `create FULL`, `update FULL` (no delete — intentional for audit trail)

---

### Group

**Table**: `group` (SCHEMAFULL) in `remex` DB

| Field | Type | Notes |
|-------|------|-------|
| `group_name` | `string` | |
| `members` | `array<record<client>>` | Default `[]` |
| `created_at` | `datetime` | READONLY |
| `updated_at` | `datetime` | READONLY |

**Audit**: `DEFINE EVENT audit_group` fires on CREATE/UPDATE/DELETE

---

### AuditLog

**Table**: `audit_log` (SCHEMAFULL) in `remex` DB

| Field | Type | Notes |
|-------|------|-------|
| `table_name` | `string` | `"job"`, `"client"`, `"group"` |
| `record_id` | `record<job | client | group>` | Polymorphic reference |
| `action` | `string` | `"CREATE"`, `"UPDATE"`, `"DELETE"` |
| `before_snapshot` | `object FLEXIBLE` | State before change |
| `after_snapshot` | `object FLEXIBLE` | State after change |
| `changed_at` | `datetime` | READONLY, default `now()` |
| `changed_by` | `option<record<user | client>>` | Who made the change |

**Permissions**: `select FULL` only (append-only)

---

### User

**Table**: `user` (SCHEMAFULL) in `remex` DB

| Field | Type | Notes |
|-------|------|-------|
| `username` | `string` | |
| `email` | `string` | UNIQUE index |
| `password` | `string` | Auto argon2-hashed via `VALUE` clause |
| `created_at` | `datetime` | READONLY |
| `updated_at` | `datetime` | READONLY |

**Access**: `DEFINE ACCESS configurator_access ON DATABASE TYPE RECORD` with dual-mode SIGNIN (email+password OR refresh_token), `DURATION FOR TOKEN 15m`

**Permissions**: `select FULL`, `create FULL`, `update WHERE id = $auth.id`, `delete NONE`

---

### RefreshToken

**Table**: `refresh_token` (SCHEMAFULL) in `remex` DB

| Field | Type | Notes |
|-------|------|-------|
| `user` | `record<user>` | Owner |
| `token` | `string` | UNIQUE index |
| `expires` | `datetime` | Expiration time |
| `active` | `bool` | Default `true` |
| `revoked_at` | `option<datetime>` | Grace period for rotation |

**Permissions**: All operations restricted to `WHERE user = $auth.id`

---

### Config

**Database**: `config` (separate from `remex`)

| Table | Schema | Fields |
|-------|--------|--------|
| `config` | SCHEMALESS | `setting_key`, `setting_value`, timestamps |
| `global_config` | SCHEMAFULL | `setting_key`, `setting_value` (object FLEXIBLE) |
| `user_config` | SCHEMAFULL | `user_id` (record\<user\>, indexed), `setting_key`, `setting_value` |

## Key Shared Types

| Type | Purpose |
|------|---------|
| `RawPacket` | 128-byte wire format array |
| `Packet` | Fragmented message unit with header + payload |
| `Message` | String + packets wrapper with fragmentation/reassembly |
| `MessageContents` | Command/Secret/Log classification by prefix |
| `ClientRequest` | Endpoint→Server protocol messages |
| `ServerResponse` | Server→Endpoint protocol messages |
| `DisconnectReason` | Disconnect cause enumeration |
| `DbError` | Database error enumeration |
| `DbOperator<T, U>` | CRUD trait for all models |
| `BearerGrantResponse` | SurrealDB bearer token response struct |
| `DbClients` | Local + remote DB connection holder |
| `SessionMap<T>` | Generic session registry |
| `JobType` | Instant / Scheduled / Recurring enum |
| `Enabled` | Draft / Enabled / Disabled enum |
| `ExecutionStatus` | Pending/Running/Completed/Failed/TimedOut (job) and Running/Completed/Failed/Cancelled/TimedOut (execution) |

## Design Decisions

### Heapless Collections
`Packet` uses `heapless::Vec` (`StackVec`) for its 126-byte payload buffer. This avoids heap allocation entirely, making the packet system suitable for constrained environments (embedded, no_std). The endpoint benefits from this when processing incoming messages.

### Mirror-Image Codecs
`ClientCodec` and `ServerCodec` are mirror images — what one encodes, the other decodes. Both use the same AES key, nonce generation, and framing format. This ensures protocol compatibility without duplicated logic.

### Computed Fields
The `execution_status` field on `job` is COMPUTED — derived from execution records at query time rather than stored. This ensures the status is always accurate without requiring application-level synchronization.

### Audit Events via SurrealDB
Audit logging uses `DEFINE EVENT` triggers that fire automatically on CREATE/UPDATE/DELETE. This ensures the audit trail is always consistent with the actual data changes, even if changes come from different sources (configurator, endpoint, direct DB access).

### Separate Config Database
The `config` database is separate from the `remex` database. This isolates UI preferences and user configuration from operational data, allowing independent backup, migration, and access control.

### DbOperator Pattern
All models implement the same `DbOperator<T, U>` trait, providing a uniform CRUD interface. The endpoint's local SurrealKV database uses the same trait as the server's remote database, enabling identical query patterns across both.
