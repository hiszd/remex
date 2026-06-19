# Remex Configurator — Architecture & Operation

## Purpose

The **Remex Configurator** is a Vue.js 3 single-page application (SPA) that serves as the administrative web interface for the remex system. It allows end users to:

- **Create, view, edit, and delete jobs** — shell commands to be executed on edge clients
- **Manage client devices** — view registered clients, their connection status, and assigned jobs
- **Organize clients into groups** — for bulk job assignment
- **Monitor job execution status** — Pending, Running, Completed, Failed, TimedOut
- **Assign jobs to individual clients or groups**

Critically, the configurator connects **directly to SurrealDB** via WebSocket — there is no REST API middleware. The server's web API was removed entirely as an architectural simplification.

## Architecture Stack

| Layer | Technology |
|-------|-----------|
| Framework | Vue 3.5 with Composition API (`<script setup>`) |
| Routing | Vue Router 5 with Web History mode |
| Data Fetching/Caching | TanStack Vue Query 5 |
| Database SDK | SurrealDB JS SDK 2.0.3 (WebSocket) |
| Build Tool | Vite 7 |
| Language | TypeScript 5.9 |
| Styling | SCSS with CSS custom properties (design tokens) |
| Date Formatting | Moment.js |

## Authentication System

### Dual Token Architecture

Three tokens stored in `sessionStorage`:
- **Access token** — JWT with 15-minute expiry
- **Email** — User's email for loading the user record
- **Refresh token** — Server-side token stored in `refresh_token` DB table (7-day expiry)

### Session Lifecycle

1. **Login/Signup** — Uses SurrealDB's `signin()`/`signup()` with `configurator_access` record access. Password is auto-hashed by SurrealDB's `VALUE crypto::argon2::generate($value)` field clause.
2. **Session restoration** — On app startup, `tryRestoreSession()` re-authenticates via refresh token, loads user record, and proactively rotates the token.
3. **Token rotation** — `rotateRefreshToken()` re-authenticates with old token, invalidates it (`active = false, revoked_at = time::now()`), creates a new token record with 60-second grace period for network failures.
4. **Logout** — Revokes refresh token server-side, clears local state, invalidates DB session.

### Global Auth Guard

Three protective mechanisms run globally:

1. **Inactivity timeout (15 min)** — Monitors `keydown`, `click`, `touchstart` events. Throttled to once per 60 seconds. After 15 min of inactivity: revokes session server-side, redirects to `/login`.
2. **Proactive token rotation** — Decodes JWT `exp` claim, schedules rotation 45 seconds before access token expiry.
3. **Vue Query error handler** — Subscribes to query/mutation caches, detects auth-related errors, clears auth and redirects on failure.

## Pages & Routes

| Route | Component | Purpose |
|-------|-----------|---------|
| `/login` | `LoginView.vue` | Email/password sign-in |
| `/register` | `RegisterView.vue` | Username/email/password registration |
| `/` | `DashboardView.vue` | System overview with stats cards and recent jobs |
| `/jobs` | `JobsView.vue` | List all jobs in card grid |
| `/jobs/new` | `CreateJobView.vue` | Job creation form (name, shell, command, timeout, state) |
| `/jobs/:id` | `JobDetailsView.vue` | View/edit/delete job, manage assignments |
| `/clients` | `ClientsView.vue` | List all registered clients |
| `/clients/:id` | `ClientDetailsView.vue` | View client info, assigned jobs, groups |
| `/groups` | `GroupsView.vue` | List all client groups |
| `/groups/new` | `CreateGroupView.vue` | Group creation form |
| `/groups/:id` | `GroupDetailsView.vue` | View/edit/delete group, manage members, assigned jobs |

## Data Flow

### Direct DB Access

All CRUD operations go directly against SurrealDB:
```typescript
// Read
const result = await client.select<Job>(new Table("job"))

// Create
const result = await client.create<Job>(new Table("job")).content(data)

// Update
const result = await client.update<Job>(rid(id)).merge(data)

// Delete
await client.delete(rid(id))
```

### Vue Query Caching

Every view uses `useQuery` with consistent patterns:
```typescript
const { data: jobs, isLoading, isError } = useQuery({
  queryKey: ["jobs"],
  queryFn: () => getJobs(client),
  select: (data) => Object.freeze(data),  // Prevents Vue reactivity overhead
})
```

Mutations invalidate specific query keys:
```typescript
const mutation = useMutation({
  mutationFn: (data) => updateJob(client, jobId, data),
  onSuccess: () => queryClient.invalidateQueries({ queryKey: ["job", jobId] }),
})
```

## Key Components

| Component | Purpose |
|-----------|---------|
| **AppSidebar.vue** | Collapsible navigation with Home/Jobs/Groups/Clients links, user avatar, logout button. State persisted to localStorage. |
| **QueryState.vue** | Generic wrapper for loading/error/empty states. Slots actual content when data is ready. |
| **MultiSelector.vue** | Assignment management with view/edit modes. Shows selectable items (clients, groups) with nested member lists. |
| **ConfirmationModal.vue** | Generic confirmation dialog with Teleport, overlay click-to-dismiss, danger variant. |
| **Job.vue / Client1.vue / Group.vue** | Card components for list views with status badges and detail links. |

## Design System

All global styles in `App.vue` with comprehensive design tokens:
- **Color scales**: `--text-50` through `--text-950`, `--background-50` through `--background-950`, `--accent-50` through `--accent-950`
- **Light/dark themes** via `@media (prefers-color-scheme)`
- **Button classes**: `.btn-primary`, `.btn-secondary`, `.btn-danger`, `.btn-ghost`
- **Card class**: `.card` with background-300, 1rem border-radius
- **Status badges**: `.status-badge` (pending/running/completed/failed/timedout), `.state-badge` (draft/enabled/disabled)
- **Layout classes**: `.page`, `.page-header`, `.info-grid`, `.app-layout`
- **Modal classes**: `.modal-overlay` with `backdrop-filter: blur(4px)`

## Key Design Decisions

1. **No REST API** — Direct SurrealDB WebSocket connection eliminates middleware complexity. Uses SurrealDB's built-in record access authentication.

2. **`Object.freeze()` on query results** — Prevents Vue from making large DB result objects deeply reactive, improving performance.

3. **Computed `execution_status`** — Derived from execution records at query time in SurrealDB, not stored. Always accurate without application-level sync.

4. **Object-typed enums** — SurrealDB has no native enum type. Uses single-key objects: `{ Pending: {} }`, `{ Enabled: {} }`, etc. TypeScript types mirror this pattern.

5. **Snake_case DB fields** — All views use actual DB field names (snake_case) rather than camelCase conversions. `FIELD_LABELS` map provides human-readable display names.
