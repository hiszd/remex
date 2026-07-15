# AGENTS.md

## Agent Orientation

This file is the single entry point for AI agents working in the `remex` repository. It is intentionally a thin navigation hub: it points you to the focused guide that covers your current task, the per-crate architecture documents that describe each executable, and the ubiquitous-language glossary in `CONTEXT.md`.

Start with the **Task Index** below to find the right guide for the work you are doing. Each guide contains the detailed conventions, tables, code snippets, and examples that used to live here. When you change architecture, schema, components, or build steps, update both the relevant guide and the source-of-truth file it describes. The **Docs Hygiene** checklist at the end of this file still applies; just follow the links into the new guide files.

If a topic is not in the Task Index, check the **Quick Links** table and `CONTEXT.md` for the canonical vocabulary before introducing new terms.

## Quick Links

| Document | Purpose |
|---|---|
| [`CONTEXT.md`](./CONTEXT.md) | Ubiquitous language, glossary, and cross-cutting domain context |
| [`core/CORE_ARCHITECTURE.md`](./core/CORE_ARCHITECTURE.md) | Shared library architecture, models, migrations, `DbOperator` |
| [`endpoint/ENDPOINT_ARCHITECTURE.md`](./endpoint/ENDPOINT_ARCHITECTURE.md) | Endpoint actor design, offline sync, job execution |
| [`server/SERVER_ARCHITECTURE.md`](./server/SERVER_ARCHITECTURE.md) | Transitional migration utility / legacy TCP listener |
| [`configurator/CONFIGURATOR_ARCHITECTURE.md`](./configurator/CONFIGURATOR_ARCHITECTURE.md) | Vue.js configurator architecture, auth, SDK usage |
| [`docs/agent-guide/DATABASE_PATTERNS.md`](./docs/agent-guide/DATABASE_PATTERNS.md) | Migrations, access control, schema design, SurrealQL patterns |
| [`docs/agent-guide/ENDPOINT_INTERNALS.md`](./docs/agent-guide/ENDPOINT_INTERNALS.md) | Endpoint actors, messages, seam functions, local cache |
| [`docs/agent-guide/CONFIGURATOR_UI.md`](./docs/agent-guide/CONFIGURATOR_UI.md) | Global CSS classes, icon library, configurator design system |
| [`docs/agent-guide/TESTING.md`](./docs/agent-guide/TESTING.md) | Test conventions, seam tests, `DbOperator`, in-memory adapter |
| [`docs/agent-guide/PITFALLS.md`](./docs/agent-guide/PITFALLS.md) | SurrealDB / SDK / Vue Query pitfalls, lessons learned, error handling |
| [`docs/agents/issue-tracker.md`](./docs/agents/issue-tracker.md) | How issues are tracked with `gh` |
| [`docs/agents/triage-labels.md`](./docs/agents/triage-labels.md) | Canonical triage label vocabulary |
| [`docs/agents/domain.md`](./docs/agents/domain.md) | Multi-context monorepo layout |

## Directory Structure

- `/core` — Shared library for all remex executables (models, migrations, `DbOperator`, actor base types, crypto utilities)
- `/server` — Transitional migration utility. Connects to SurrealDB, runs core migrations, and still starts a legacy TCP listener on `127.0.0.1:4269` using `RemexServer`/`RemexSession` actors; **not** used for endpoint enrollment
- `/endpoint` — Edge client that connects directly to SurrealDB, caches jobs/executions locally, and runs scheduled jobs
- `/macros` — proc-macro crate for derive macros
- `/configurator` — Standalone Vue.js web application that connects directly to SurrealDB
- `/docs` — Architecture documents, ADRs, and agent guides

## Systems Design

`remex_core` is the shared library that holds all logic for the server and endpoint.

`remex_endpoint` is the edge client. It authenticates directly to the core SurrealDB database, maintains a local SurrealKV cache for offline operation, and spawns background tasks to sync executions and run jobs.

`remex_configurator` is the end-user web application. It connects **directly to the core SurrealDB database** using SurrealDB's built-in record authentication. User preferences and UI configuration are stored in a separate `config` database.

`remex_server` is a transitional migration utility. It connects to SurrealDB, runs the core migrations, and still starts a legacy TCP listener on `127.0.0.1:4269` using `RemexServer`/`RemexSession` actors from `core/src/actors/`. The endpoint does **not** connect to it; endpoints authenticate directly to SurrealDB.

## Key Dependencies

| Dependency | Version | Purpose |
|---|---|---|
| actix | 0.13 | Actor system for async message passing |
| surrealdb | 3 | Database with kv-surrealkv and protocol-ws |
| tokio | 1.38.1 | Async runtime |
| aes-gcm | 0.10.3 | Encryption for TCP socket communication |
| chrono | 0.4.38 | Date/time handling |
| tracing | 0.1.40 | Structured logging |
| uuid | 1.6 | Unique identifiers (v4, v7) |

## Task Index

| If you are working on... | Read this |
|---|---|
| Adding/modifying a SurrealDB table or access method | [`docs/agent-guide/DATABASE_PATTERNS.md`](./docs/agent-guide/DATABASE_PATTERNS.md) + relevant migration file |
| Changing endpoint job execution, scheduling, or sync | [`docs/agent-guide/ENDPOINT_INTERNALS.md`](./docs/agent-guide/ENDPOINT_INTERNALS.md) + `endpoint/src/async_tasks/jobs/execution.rs` |
| Adding/modifying endpoint actors or messages | [`docs/agent-guide/ENDPOINT_INTERNALS.md`](./docs/agent-guide/ENDPOINT_INTERNALS.md) + `endpoint/src/async_tasks.rs` |
| Adding a configurator view/component or global style | [`docs/agent-guide/CONFIGURATOR_UI.md`](./docs/agent-guide/CONFIGURATOR_UI.md) + [`configurator/CONFIGURATOR_ARCHITECTURE.md`](./configurator/CONFIGURATOR_ARCHITECTURE.md) |
| Changing configurator auth/session behavior | [`docs/agent-guide/DATABASE_PATTERNS.md`](./docs/agent-guide/DATABASE_PATTERNS.md) access section + `configurator/src/lib/auth.ts` |
| Writing tests for DB-dependent logic | [`docs/agent-guide/TESTING.md`](./docs/agent-guide/TESTING.md) |
| Debugging a SurrealQL / SDK / Vue Query bug | [`docs/agent-guide/PITFALLS.md`](./docs/agent-guide/PITFALLS.md) |

## Source of Truth

| Area | Source of truth |
|---|---|
| Job schema | `core/src/db/model/jobs.rs` |
| Client schema + endpoint access | `core/src/db/model/clients.rs` |
| Execution schema | `core/src/db/model/executions.rs` |
| Endpoint actor messages | `endpoint/src/async_tasks.rs` |
| Endpoint seam functions | `endpoint/src/async_tasks/jobs/execution.rs`, `endpoint/src/async_tasks/jobs/sync.rs` |
| Local cache schema | `endpoint/src/db.rs` + migration modules |
| Configurator auth | `configurator/src/lib/auth.ts` |
| Configurator routes / route meta | `configurator/src/router/index.ts` |
| Global CSS classes | `configurator/src/App.vue` |
| Icon components | `configurator/src/components/icons/` |
| `DbOperator` trait + adapters | `core/src/db/mod.rs`, `core/src/db/adapters.rs` |

## Docs Hygiene

`AGENTS.md` and the `docs/agent-guide/*.md` files are unified entry points, not dumping grounds for ephemeral todos. After any architecture, schema, component, or build change, run this checklist to keep docs consistent:

- [ ] Does the change affect the directory structure or crate roles? Update [Directory Structure](#directory-structure) and [Systems Design](#systems-design).
- [ ] Did an actor, message, task, or seam change? Update [`docs/agent-guide/ENDPOINT_INTERNALS.md`](./docs/agent-guide/ENDPOINT_INTERNALS.md) and [`endpoint/ENDPOINT_ARCHITECTURE.md`](./endpoint/ENDPOINT_ARCHITECTURE.md).
- [ ] Did the SurrealDB schema change (tables, fields, indexes, events, access methods)? Update [`docs/agent-guide/DATABASE_PATTERNS.md`](./docs/agent-guide/DATABASE_PATTERNS.md) and the relevant migration file; copy the authoritative snippet from code, not memory.
- [ ] Did a configurator component or view change? Update [`docs/agent-guide/CONFIGURATOR_UI.md`](./docs/agent-guide/CONFIGURATOR_UI.md) and [`configurator/CONFIGURATOR_ARCHITECTURE.md`](./configurator/CONFIGURATOR_ARCHITECTURE.md).
- [ ] Did auth behavior change (token lifetime, storage, refresh flow)? Update the auth notes in [`docs/agent-guide/DATABASE_PATTERNS.md`](./docs/agent-guide/DATABASE_PATTERNS.md) and [`docs/agent-guide/CONFIGURATOR_UI.md`](./docs/agent-guide/CONFIGURATOR_UI.md).
- [ ] Did error-handling or testing conventions change? Update [`docs/agent-guide/PITFALLS.md`](./docs/agent-guide/PITFALLS.md) and [`docs/agent-guide/TESTING.md`](./docs/agent-guide/TESTING.md).
- [ ] Did you add/remove dependencies or change versions? Update [Key Dependencies](#key-dependencies).
- [ ] Are there deleted design docs or ADRs still referenced anywhere? Remove stale links.
- [ ] Run `cargo xtask doc-lint` (or invoke the `doc_lint` tool) and fix all violations before considering the change complete.
- [ ] Run `cargo test` and the configurator type-check / build to confirm the docs match reality.

When in doubt, prefer deleting obsolete detail over leaving a contradiction.

## Agent Infrastructure

Issues are tracked in GitHub using the `gh` CLI. External PRs are not treated as a triage surface. See [`docs/agents/issue-tracker.md`](./docs/agents/issue-tracker.md).

Using default canonical triage labels. See [`docs/agents/triage-labels.md`](./docs/agents/triage-labels.md).

Multi-context monorepo layout. See [`docs/agents/domain.md`](./docs/agents/domain.md).
