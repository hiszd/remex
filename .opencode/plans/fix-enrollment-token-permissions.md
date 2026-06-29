# Fix enrollment token SELECT permission for SIGNUP

## Problem

The `endpoint_access` SIGNUP block in `clients.rs` does `SELECT * FROM enrollment_token WHERE token_hash = crypto::sha256($token)` to look up the enrollment token. But the `enrollment_token` table has:

```surrealql
PERMISSIONS FOR select, create, update WHERE $auth.id IN (SELECT id FROM user)
```

During SIGNUP, there is **no `$auth.id`** — the endpoint hasn't authenticated yet. So the SELECT is denied, `$tok = NONE`, and the SIGNUP throws `'Invalid or expired enrollment token'`.

## Fix

In `core/src/db/model/enrollment_token.rs`, change line 43:

**Before:**
```surrealql
DEFINE TABLE IF NOT EXISTS enrollment_token SCHEMAFULL
  PERMISSIONS FOR select, create, update WHERE $auth.id IN (SELECT id FROM user),
            FOR delete NONE;
```

**After:**
```surrealql
DEFINE TABLE IF NOT EXISTS enrollment_token SCHEMAFULL
  PERMISSIONS FOR select FULL,
            FOR create WHERE $auth.id IN (SELECT id FROM user),
            FOR update WHERE $auth.id IN (SELECT id FROM user),
            FOR delete NONE;
```

No other files need changes. After editing, re-run the server migration against the remote DB to apply the updated schema.

## Security

`FOR select FULL` allows unauthenticated clients to query `enrollment_token`. This is acceptable because:
- `token_hash` is SHA256 (irreversible — raw token can't be derived)
- Exposed metadata: `created_at`, `single_use`, `valid`, `expires_at` — low sensitivity
- Without the raw token string, knowing the hash is useless
- The SIGNUP block already needed this access and was blocked
