# Configurator UI

## CSS/SCSS Styles

All styles should use colors from the themes, and not hard-coded colors. If a theme color needs to be added, it should be added to the theme file.

### Global Design System (App.vue)

All common UI patterns are defined as global styles in `App.vue`. Use these classes in templates instead of redefining styles in component scoped styles.

#### CSS Variables
- `--color-border`: Border color (maps to `--background-400`)

#### Buttons
- `.btn-primary`: Accent-colored action button
- `.btn-secondary`: Outlined accent button
- `.btn-danger`: Red danger/delete button
- `.btn-ghost`: Subtle bordered button

All buttons share: `padding: 0.625rem 1.25rem`, `border-radius: 0.5rem`, `font-size: 0.875rem`, `font-weight: 600/700`, hover/disabled states.

#### Cards
- `.card`: Standard card with `background: var(--background-300)`, `border-radius: 1rem`, `padding: 1.5rem`, subtle shadow

#### Badges
- `.status-badge`: For execution status (pending, running, completed, failed, timedout)
- `.state-badge`: For enabled state (draft, enabled, disabled)
Both share: `padding: 0.125rem 0.625rem`, `border-radius: 9999px`, `font-size: 0.75rem`, `font-weight: 600`

#### Forms
- `.form-input`: Standard input/textarea/select styling with focus ring
- `.form-label`: Uppercase label style (`font-size: 0.8rem`, `font-weight: 700`, `letter-spacing: 0.025em`)
- `.info-label`: Smaller label for read-only detail views (`font-size: 0.7rem`, `letter-spacing: 0.05em`)

#### Layout
- `.page`: Page container with `padding: 2rem 1.5rem`, `gap: 2rem`
- `.page-header`: Header container
- `.page-title`: List page titles (`font-size: 1.75rem`, `font-weight: 800`)
- `.page-subtitle`: Subtitle text (`font-size: 0.9rem`, `color: var(--text-500)`)
- `.back-link`: Navigation back link
- `.header-main`: Detail page header with title + actions
- `.title-group`: Title + record ID grouping
- `.section-header`: Section header with h2 + optional action button
- `.info-grid`: Responsive grid for detail view info items
- `.info-item`: Single info item with label + value

#### Utilities
- `.empty-state`: Empty list/content placeholder
- `.state-card`: Loading/error/empty state container
- `.monospace`: Monospace text styling
- `.spinner` + `@keyframes spin`: Loading spinner
- `.details-btn`: Icon-only "view details" button
- `.assignment-list`, `.assignment-item`, `.assignment-badges`, `.assignment-name`: Assignment list patterns

#### Modals
- `.modal-overlay`: Fixed overlay with `backdrop-filter: blur(4px)`, `z-index: 9999`
- `.modal-content`: Centered modal card
- `.modal-actions`: Action button row

## Icon Library

**Location**: `/configurator/src/components/icons/`

**Naming Convention**: `Icon[Name].vue` (PascalCase)

**Guidelines for Creating Icons**:
- Use `viewBox="0 0 [width] [height]"` for scalable icons
- Default sizes: 16px for button icons, 20px for standard icons
- Use `stroke="currentColor"` and `fill="none"` for line-based icons
- Use `stroke-width="1.5"` for consistent line weight
- Use `stroke-linecap="round"` and `stroke-linejoin="round"` for smooth corners
- Always use `currentColor` to inherit text color and support light/dark themes

**Using Icons in Components**:
- Icon containers must have a theme-aware color set (e.g., `color: var(--text)`)
- This ensures icons automatically adapt to light/dark mode
- Example:
  ```scss
  .icon-container {
    color: var(--text);
    // Icons will inherit this color via currentColor
  }
  ```

**Available Icons**:

### IconViewDetails.vue (16x16)
Document icon for "view details" actions.

```vue
<template>
  <svg width="16" height="16" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
    <rect x="3" y="2" width="10" height="12" rx="1"/>
    <path d="M6 6h4M6 9h4"/>
  </svg>
</template>
```

### IconGroup.vue (20x20)
Overlapping people icon for groups.

```vue
<template>
  <svg width="20" height="20" viewBox="0 0 20 20" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
    <circle cx="7" cy="7" r="3"/>
    <path d="M2 17v-1a4 4 0 0 1 4-4h2a4 4 0 0 1 4 4v1"/>
    <circle cx="14" cy="8" r="2"/>
    <path d="M18 17v-1a3 3 0 0 0-3-3h-1"/>
  </svg>
</template>
```

### IconClient.vue (20x20)
Monitor icon for clients.

```vue
<template>
  <svg width="20" height="20" viewBox="0 0 20 20" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
    <rect x="3" y="4" width="14" height="10" rx="1"/>
    <path d="M8 17h4M10 14v3"/>
  </svg>
</template>
```

### IconJob.vue (16x16)
Briefcase icon for jobs.

```vue
<template>
  <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
    <rect x="2" y="7" width="20" height="14" rx="2" ry="2" />
    <path d="M16 21V5a2 2 0 00-2-2h-4a2 2 0 00-2 2v16" />
  </svg>
</template>
```

**Usage Example**:

```vue
<script setup>
import IconViewDetails from '@/components/icons/IconViewDetails.vue'
</script>

<template>
  <button>
    <IconViewDetails />
    View Details
  </button>
</template>
```

## Configurator Design System

The configurator uses a blue corporate design system with a collapsible sidebar + top bar layout.

### Layout Architecture

| Component | Role |
|---|---|
| `App.vue` | Root — provides `DesignSidebar` + `DesignTopBar` for authenticated routes; auth pages (login/register) use a full-screen layout |
| `DesignSidebar.vue` | Collapsible sidebar with icon + label nav items. Auto-collapses ≤768px. Dark bg: `#334155`. Blue accent active state with left border indicator |
| `DesignTopBar.vue` | Shows page title (from `route.meta.title`), hamburger toggle, optional search input |

Current shared components also include `Client.vue`, `ClientTree.vue`, `MultiSelector.vue`, `ConfirmationModal.vue`, and `QueryState.vue`.

### Route Meta

Each route defines `meta.title` used by `DesignTopBar`:

| Route | `meta.title` |
|---|---|
| `/` | "Dashboard" |
| `/jobs` | "Jobs" |
| `/jobs/new` | "Create Job" |
| `/jobs/:id` | "Job Details" |
| `/groups` | "Groups" |
| `/groups/new` | "Create Group" |
| `/groups/:id` | "Group Details" |
| `/clients` | "Clients" |
| `/clients/:id` | "Client Details" |
| `/tokens` | "Tokens" |
| `/tokens/new` | "Create Token" |
| `/tokens/:id` | "Token Details" |
| `/executions/:id` | "Execution Details" |

### Global CSS Classes (defined in App.vue)

**Layout:** `.page`, `.page-title`, `.page-subtitle`, `.header-main`, `.title-group`, `.header-actions`, `.section-header`, `.section-header-row`, `.section-actions`, `.back-link`, `.design-layout`, `.design-content`, `.design-main`

**Data display:** `.data-table`, `.data-table-card`, `.stats-grid`, `.stat-card`, `.stat-card-header`, `.stat-card-label`, `.stat-card-value`, `.stat-card-change`, `.stat-icon`

**Badges:** `.status-badge` (pending/running/completed/failed/timedout), `.state-badge` (draft/enabled/disabled)

**Forms:** `.form-input`, `.form-label`, `.info-label`, `.info-grid`, `.info-item`

**Buttons:** `.btn-primary` (blue fill), `.btn-secondary` (blue-tinted fill), `.btn-danger` (vibrant red fill), `.btn-ghost` (slate fill)

**Utilities:** `.card`, `.back-link`, `.empty-state`, `.empty-state-block`, `.empty-state-icon`, `.state-card`, `.monospace`, `.spinner`, `.modal-overlay`, `.modal-content`, `.modal-actions`, `.assignment-list`, `.assignment-item`, `.assignment-badges`, `.assignment-name`, `.details-btn`, `.search-input`, `.view-toggle`, `.hamburger-btn`, `.sidebar-overlay`, `.top-bar`, `.top-bar-left`, `.top-bar-right`, `.top-bar-title`, `.top-bar-breadcrumbs`

### View Patterns

| Pattern | Files |
|---|---|
| **List with view toggle** (table/grid) | `JobsView.vue`, `GroupsView.vue`, `ClientsView.vue`, `TokensView.vue` |
| **Detail with info-grid + sections** | `JobDetailsView.vue`, `GroupDetailsView.vue`, `ClientDetailsView.vue`, `TokenDetailsView.vue`, `ExecutionDetailsView.vue` |
| **Create form** (back-link + card form) | `CreateJobView.vue`, `CreateGroupView.vue`, `CreateTokenView.vue` |
| **Dashboard** (stat cards + recent data-table) | `DashboardView.vue` |

### View Toggle Pattern

List views implement a table/grid toggle:
```vue
const viewMode = ref<"table" | "grid">("table")
```

When `viewMode === "table"`, render `data-table-card > data-table`. When `viewMode === "grid"`, render a `stats-grid` of clickable stat-card elements.

### Auth Storage

The configurator stores the JWT access token, authenticated email, and server-side refresh token in **`sessionStorage`** (not `localStorage`). The access token expires after 15 minutes (`15m`); session restore uses the `refresh_token` table.

### Data Gaps (not currently in schema)

| Data Point | Where Needed | Schema Gap |
|---|---|---|
| Online/offline status per client | Dashboard stat card, client list | `last_seen` exists but no live status |
| Job execution count | Dashboard, client details, group details | Requires aggregation query on `execution` table |
| Job last execution time | Dashboard, job details | Not stored on job — derived from executions |
| Execution duration | Job details | `execution_start`/`execution_end` exist but not surfaced |
| Job description | Job detail | No field exists |
| Job tags/labels | Job list filter | No field exists |
| Next scheduled run | Job detail | Cron/schedule not yet in schema |
| Client IP / location | Client details | `connection_history` has ip but not surfaced |

### Removed Files (post-redesign)

| File | Reason |
|---|---|
| `AppSidebar.vue` | Replaced by `DesignSidebar.vue` |
| `Job.vue` | Replaced by data-table rows |
| `Group.vue` | Replaced by data-table rows |
| `Client1.vue` | Replaced by data-table rows |
| `DesignPrototype.vue` | Design routes removed |
| `hero-card.scss` | Unused styles |
