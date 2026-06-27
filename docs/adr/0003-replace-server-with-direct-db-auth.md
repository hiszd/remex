# Replace the TCP Server with Direct Database Authentication

The Server component (encrypted TCP socket, sign-up/sign-in, bearer token provisioning) has been removed. The Endpoint now connects directly to SurrealDB. Authentication is handled entirely by the database's `DEFINE ACCESS` rules: enrollment tokens for first-time sign-up, then `hardware_hash` + `secret` for ongoing SIGNIN. Granular `PERMISSIONS` on each table limit the Endpoint to only the operations it needs (SELECT job/group, SELECT own client, CREATE/UPDATE execution). The `blocked` boolean on the `client` table provides immediate admin revocation. The `secret` field's `crypto::argon2::generate($value)` value transform prevents plaintext storage.

## Considered Options

- **Keep the Server as a thin proxy**: More infrastructure to operate, single point of failure, no clear security benefit once permissions are database-native.
- **Bearer FOR RECORD only**: Stolen bearer token is valid until expiry with no revocation mechanism short of changing DB-level access definitions. No per-request revocation check.
- **Hybrid (Server + DB auth)**: Adds operational complexity without closing any threat surface that the chosen approach doesn't handle.

## Consequences

- The Server crate can be removed from the workspace entirely (TCP handshake, encryption, ping/pong — all gone).
- Every DB query from the endpoint is implicitly authorization-checked via permissions. No Server-side logic to bypass.
- The Configurator and Endpoint share the same connection pattern (direct to SurrealDB), just with different access credentials and permission scopes.
- New Endpoint deployment requires only an enrollment token — no Server coordination.
- The `client` table name stays as `client` for now. The glossary distinguishes `Endpoint` (software agent) from `Host` (physical machine), but the table rename is deferred.
