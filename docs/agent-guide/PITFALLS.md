# Pitfalls

## SurrealDB Pitfalls

### `to_sql()` Adds the Table Prefix

`RecordId::to_sql()` returns `"table_name:key"`, not just `"key"`. This is easy to forget when constructing strings for field comparison:

```rust
let id = RecordId::new("job", "abc123");
assert_eq!(id.to_sql(), "job:abc123"); // NOT "abc123"
```

Test code often mirrors this with a helper:
```rust
fn job_sql_id(key: &str) -> String {
  format!("job:{key}")
}
```

### `UPDATE ... CONTENT` vs `UPDATE ... MERGE`

The `impl_surreal_db_operator!` macro uses different SurrealQL keywords for `create` and `update`:

- **`CREATE ... CONTENT $data`** — replaces the entire document body. Any field not present in `$data` is removed. This can break SCHEMAFULL tables if required fields are omitted.
- **`UPDATE $id MERGE $data`** — partial update. Only the fields present in `$data` are changed. The record `id` is always included in the response.

In SurrealDB v3, `UPDATE ... CONTENT` may return the document body **without** the record `id`, causing deserialization failures on structs that require an `id: RecordId` field. This is why `update` uses `MERGE`, not `CONTENT`.

### Schemafull Tables Require All Fields

SCHEMAFULL tables (all core and endpoint tables except `config`) require that any field defined via `DEFINE FIELD` must be present on `CREATE` unless it has a `DEFAULT`. For example, the `execution` table has zero optional field definitions — every field (`output`, `command`, `exit_code`, `execution_start`, `execution_end`, etc.) must be provided.

### Local and Remote Tables Share Names

The endpoint's local cache `job` table and the remote `job` table are both in `remex.remex` on their respective SurrealDB instances. They have different schemas:

| Table | Location | Schema |
|---|---|---|
| `job` (remote) | Cloud SurrealDB | `job_name`, `job_shell`, `job_command`, `job_type`, `execution_status`, `enabled`, ... |
| `job` (cache) | Local SurrealKV | `job_id` (string), `job_info` (FLEXIBLE), `completed` (bool) |

The same applies to `execution`. The endpoint's migration only defines the cache fields, so queries must match the table's actual schema on the instance being queried.

### In-Memory Adapter Has UPSERT Semantics

The in-memory adapter's `update` calls `HashMap::insert`, which **creates** a record if it doesn't exist. The real `UPDATE ... MERGE` in SurrealDB may error or return empty for non-existent records. Tests using the in-memory adapter can mask production errors where updates target missing records.

### (Configurator) `enabled` in `@tanstack/vue-query` is Evaluated Once

Passing a plain boolean like `enabled: !!job.value` evaluates at setup time and is never re-evaluated. Use a function `enabled: () => !!job.value` for reactive behavior, or restructure the query to not depend on another query's result.

**Wrong** — `enabled` is locked at `false` forever:
```ts
const { data: job } = useQuery({ queryKey: ["job", id], queryFn: () => getJob(client, id) })
const { data: execs } = useQuery({
  queryKey: ["executions", id],
  queryFn: () => getExecutionsForJob(client, job.value!.id),
  enabled: !!job.value,  // BUG: never re-evaluated
})
```

**Right** — make the executions query independent:
```ts
const { data: job } = useQuery({ queryKey: ["job", id], queryFn: () => getJob(client, id) })
const { data: execs } = useQuery({
  queryKey: ["executions", id],
  queryFn: () => getExecutionsForJob(client, rid(id)),  // uses route param directly
})
```

### (Configurator) SurrealDB JS SDK `RecordId.toString()` Double-Wraps Angle Brackets

The SDK v2.0.3's `escapeIdent` function wraps record ID values in `⟨...⟩` when they contain non-ASCII characters. If the `id` part already contains `⟨...⟩` (as with UUID-based record IDs from SurrealDB), `toString()` double-wraps them: `execution:⟨⟨uuid\⟩⟩`.

When round-tripping a RecordId through URL serialization (`router.push(\`/path/${recordId}\`)`) and back through `rid()`, the `rid()` helper must strip the outer brackets before constructing a new `RecordId`:

```ts
export function rid(id: string): RecordId {
  const sep = id.indexOf(":")
  let idPart = id.slice(sep + 1)
  if (idPart.startsWith("\u27e8") && idPart.endsWith("\u27e9")) {
    idPart = idPart.slice(1, -1)
  }
  return new RecordId(id.slice(0, sep), idPart)
}
```

Always use `rid()` (not `new RecordId(...)`) when parsing record IDs from route params or user input.

## Lessons Learned

### 1. `enabled` in Query Libraries Must Be Reactive

TanStack Vue Query v5 evaluates `enabled` once inside a `computed()` at setup time. Passing a plain boolean like `!!job.value` evaluates immediately and is never re-evaluated, even if `job.value` changes later. This creates a subtle deadlock — a query dependent on another query's data may never fire on fresh page loads when that data hasn't been cached yet.

**Always make queries independent of each other when possible.** A query's `queryFn` should derive its parameters from stable sources (route params, refs initialized before queries) rather than another query's result. If you must chain queries, pass `enabled` as a function: `enabled: () => !!job.value`.

### 2. Don't Trust SDK `toString()` for Round-Tripping

The SurrealDB JS SDK v2.0.3's `RecordId.toString()` produces a SurrealQL-compatible string that wraps UUID IDs in `⟨...⟩` via `escapeIdent`. If you parse that string back into a `RecordId` via `new RecordId(table, idPart)`, the new `RecordId`'s `toString()` will wrap the already-bracketed `id` again, producing `execution:⟨⟨uuid\⟩⟩` — a value that doesn't match any record in the database.

**The `rid()` helper exists precisely to handle this.** Always route record ID string parsing through `rid()` rather than directly calling `new RecordId()`. See the pitfall above for the implementation.

### 3. Debug with Logging, Fix by Simplifying

Both bugs were diagnosed by adding `console.log` to trace the actual values flowing through the system:
- In the execution detail view, logging `execId`, `String(recordId)`, and `recordId.id` revealed that the queried ID didn't match database records
- In the job details view, checking whether the executions query was ever firing (it wasn't) revealed the `enabled` deadlock

Once diagnosed, both fixes simplified the code — removing the `enabled` option entirely in one case, and making `rid()` more robust in the other. If a fix complicates the code, it's probably the wrong fix.

### 4. URL Round-Trips Can Corrupt Typed Values

Passing SurrealDB `RecordId` objects through URL serialization (`router.push(\`/path/${recordId}\`)`) and back (`route.params.id` → `rid()`) is inherently lossy. The SDK's string representation includes SurrealQL escaping that isn't idempotent. Prefer to pass the id as its raw parts (`table:id` without escaping) or keep a reference to the original `RecordId` object in a store/cache rather than re-parsing from the URL.

## Error Handling

### Hard Rule — Never Silently Discard Errors

There is absolutely no circumstance where errors should be silently discarded. Every error must either:
- **Log to the command line and continue execution** (with `tracing::error!`/`tracing::warn!`), or
- **Stop the program execution** if there is no appropriate fallback

**There are no exceptions.**

This means:
- ❌ **Never** use `let _ =` to swallow a `Result` or error-returning call
- ❌ **Never** use `.ok()` or `.unwrap_or_default()` to ignore errors without logging
- ✅ Use `let _ =` only for values that are genuinely not errors (e.g., `Sink` items, drop handles, or test cleanup where failure is acceptable)
- ✅ Always log the error context before continuing after a non-fatal failure

### Approaches

This codebase uses two error handling approaches:

#### thiserror

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

#### anyhow

For general application error handling with context propagation.

When adding new errors, prefer `thiserror` for domain-specific errors and `thiserror::Error` derive macro.
