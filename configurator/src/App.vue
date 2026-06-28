<script setup lang="ts">
import { watch, ref, computed } from "vue"
import { RouterView, useRoute, useRouter } from "vue-router"
import DesignSidebar from "@/components/design/DesignSidebar.vue"
import DesignTopBar from "@/components/design/DesignTopBar.vue"
import { provideSurreal } from "@/lib/surreal"
import { authReady, tryRestoreSession } from "@/lib/auth"
import { useAuthGuard, setupGlobalInactivityListeners } from "@/lib/authGuard"

const route = useRoute()
const router = useRouter()
const showApp = ref(false)
const connectionError = ref<string | null>(null)
const sidebarExpanded = ref(true)

const isAuthPage = computed(() =>
  ["login", "register"].includes(String(route.name))
)

const pageTitle = computed(() =>
  (route.meta.title as string) ?? "Remex"
)

const breadcrumbs = computed(() => {
  const name = route.name as string
  switch (name) {
    case "home":
      return [{ label: "Dashboard" }]
    case "jobs":
      return [{ label: "Jobs" }]
    case "job-details":
      return [{ label: "Jobs", path: "/jobs" }, { label: "Job Details" }]
    case "create-job":
      return [{ label: "Jobs", path: "/jobs" }, { label: "Create Job" }]
    case "groups":
      return [{ label: "Groups" }]
    case "group-details":
      return [{ label: "Groups", path: "/groups" }, { label: "Group Details" }]
    case "create-group":
      return [{ label: "Groups", path: "/groups" }, { label: "Create Group" }]
    case "clients":
      return [{ label: "Clients" }]
    case "client-details":
      return [{ label: "Clients", path: "/clients" }, { label: "Client Details" }]
    case "execution-details":
      return [{ label: "Jobs", path: "/jobs" }, { label: "Execution Details" }]
    default:
      return []
  }
})

const { client, isSuccess, isError, connect, error } = provideSurreal({
  endpoint: "ws://192.168.10.87:8090",
  params: {
    namespace: "remex",
    database: "remex",
  },
  autoConnect: false,
})

const { scheduleRotation } = useAuthGuard(client)

connect().catch(() => { })

watch(isSuccess, async (connected) => {
  if (connected) {
    await tryRestoreSession(client)
  }
})

watch(isError, (errored) => {
  if (errored) {
    connectionError.value = String(error.value ?? "Connection failed")
  }
})

watch(authReady, (ready) => {
  if (ready) {
    scheduleRotation()
    setupGlobalInactivityListeners(client, router)
    showApp.value = true
  }
})
</script>

<template>
  <div v-if="connectionError" class="loading-screen">
    <div class="error-icon">⚠</div>
    <p class="error-title">Connection failed</p>
    <p class="error-detail">{{ connectionError }}</p>
  </div>

  <div v-else-if="!showApp" class="loading-screen">
    <div class="spinner" />
    <p class="loading-text">Loading...</p>
  </div>

  <div v-else-if="!isAuthPage" class="design-layout">
    <DesignSidebar
      :expanded="sidebarExpanded"
      :active-item="route.path"
      @navigate="router.push"
      @close="sidebarExpanded = false"
    />
    <div :class="['sidebar-overlay', { visible: sidebarExpanded }]" @click="sidebarExpanded = false" />
    <div class="design-content">
      <DesignTopBar
        :title="pageTitle"
        :breadcrumbs="breadcrumbs"
        @toggle-sidebar="sidebarExpanded = !sidebarExpanded"
      />
      <main class="design-main">
        <Suspense>
          <RouterView />
          <template #fallback>
            <div class="state-card">
              <div class="spinner" />
              <p class="state-text">Loading...</p>
            </div>
          </template>
        </Suspense>
      </main>
    </div>
  </div>

  <div v-else class="auth-layout">
    <Suspense>
      <RouterView />
      <template #fallback>
        <div class="state-card">
          <div class="spinner" />
          <p class="state-text">Loading...</p>
        </div>
      </template>
    </Suspense>
  </div>
</template>

<style lang="scss">
*,
*::before,
*::after {
  box-sizing: border-box;
  margin: 0;
  font-weight: normal;
}

body {
  margin: 0;
  background: var(--background);
  color: var(--text);
  transition: color 0.3s, background-color 0.3s;
  font-family: Inter, -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto,
    Oxygen, Ubuntu, Cantarell, "Fira Sans", "Droid Sans", "Helvetica Neue",
    sans-serif;
  font-size: 15px;
  text-rendering: optimizeLegibility;
  -webkit-font-smoothing: antialiased;
  -moz-osx-font-smoothing: grayscale;
}

@media (prefers-color-scheme: light) {
  :root {
    --text: #0f172a;
    --background: #f1f5f9;
    --primary: #1e3a5f;
    --secondary: #64748b;
    --accent: #2563eb;

    --text-50: #f8fafc;
    --text-100: #f1f5f9;
    --text-200: #e2e8f0;
    --text-300: #cbd5e1;
    --text-400: #94a3b8;
    --text-500: #64748b;
    --text-600: #475569;
    --text-700: #334155;
    --text-800: #1e293b;
    --text-900: #0f172a;
    --text-950: #020617;

    --background-50: #ffffff;
    --background-100: #f8fafc;
    --background-200: #f1f5f9;
    --background-300: #e2e8f0;
    --background-400: #cbd5e1;
    --background-500: #94a3b8;
    --background-600: #64748b;
    --background-700: #475569;
    --background-800: #334155;
    --background-900: #1e293b;
    --background-950: #0f172a;

    --primary-50: #eff6ff;
    --primary-100: #dbeafe;
    --primary-200: #bfdbfe;
    --primary-300: #93c5fd;
    --primary-400: #60a5fa;
    --primary-500: #3b82f6;
    --primary-600: #2563eb;
    --primary-700: #1d4ed8;
    --primary-800: #1e3a5f;
    --primary-900: #1e3a5f;
    --primary-950: #172554;

    --secondary-50: #f1f5f9;
    --secondary-100: #e2e8f0;
    --secondary-200: #cbd5e1;
    --secondary-300: #94a3b8;
    --secondary-400: #64748b;
    --secondary-500: #475569;
    --secondary-600: #334155;
    --secondary-700: #1e293b;
    --secondary-800: #0f172a;
    --secondary-900: #020617;
    --secondary-950: #000000;

    --accent-50: #eff6ff;
    --accent-100: #dbeafe;
    --accent-200: #bfdbfe;
    --accent-300: #93c5fd;
    --accent-400: #60a5fa;
    --accent-500: #3b82f6;
    --accent-600: #2563eb;
    --accent-700: #1d4ed8;
    --accent-800: #1e40af;
    --accent-900: #1e3a8a;
    --accent-950: #172554;

    --status-pending-bg: #fef9c3;
    --status-pending-text: #854d0e;
    --status-running-bg: #dbeafe;
    --status-running-text: #1e40af;
    --status-completed-bg: #dcfce7;
    --status-completed-text: #166534;
    --status-failed-bg: #fee2e2;
    --status-failed-text: #991b1b;
    --status-timedout-bg: #f3e8ff;
    --status-timedout-text: #6b21a8;

    --danger-bg: #fee2e2;
    --danger-text: #dc2626;
  }
}

@media (prefers-color-scheme: dark) {
  :root {
    --text: #e2e8f0;
    --background: #0f172a;
    --primary: #94a3b8;
    --secondary: #475569;
    --accent: #3b82f6;

    --text-50: #020617;
    --text-100: #0f172a;
    --text-200: #1e293b;
    --text-300: #334155;
    --text-400: #475569;
    --text-500: #64748b;
    --text-600: #94a3b8;
    --text-700: #cbd5e1;
    --text-800: #e2e8f0;
    --text-900: #f1f5f9;
    --text-950: #f8fafc;

    --background-50: #020617;
    --background-100: #0f172a;
    --background-200: #1e293b;
    --background-300: #334155;
    --background-400: #475569;
    --background-500: #64748b;
    --background-600: #94a3b8;
    --background-700: #cbd5e1;
    --background-800: #e2e8f0;
    --background-900: #f1f5f9;
    --background-950: #f8fafc;

    --primary-50: #172554;
    --primary-100: #1e3a8a;
    --primary-200: #1e40af;
    --primary-300: #1d4ed8;
    --primary-400: #2563eb;
    --primary-500: #3b82f6;
    --primary-600: #60a5fa;
    --primary-700: #93c5fd;
    --primary-800: #bfdbfe;
    --primary-900: #dbeafe;
    --primary-950: #eff6ff;

    --secondary-50: #000000;
    --secondary-100: #020617;
    --secondary-200: #0f172a;
    --secondary-300: #1e293b;
    --secondary-400: #334155;
    --secondary-500: #475569;
    --secondary-600: #64748b;
    --secondary-700: #94a3b8;
    --secondary-800: #cbd5e1;
    --secondary-900: #e2e8f0;
    --secondary-950: #f1f5f9;

    --accent-50: #172554;
    --accent-100: #1e3a8a;
    --accent-200: #1e40af;
    --accent-300: #1d4ed8;
    --accent-400: #2563eb;
    --accent-500: #3b82f6;
    --accent-600: #60a5fa;
    --accent-700: #93c5fd;
    --accent-800: #bfdbfe;
    --accent-900: #dbeafe;
    --accent-950: #eff6ff;

    --status-pending-bg: #422006;
    --status-pending-text: #fbbf24;
    --status-running-bg: #1e3a5f;
    --status-running-text: #60a5fa;
    --status-completed-bg: #052e16;
    --status-completed-text: #22c55e;
    --status-failed-bg: #450a0a;
    --status-failed-text: #ef4444;
    --status-timedout-bg: #3b0764;
    --status-timedout-text: #d8b4fe;

    --danger-bg: #450a0a;
    --danger-text: #fca5a5;
  }
}

:root {
  --color-background: var(--background);
  --color-border: var(--background-400);
  --sidebar-bg: var(--background-950);
  --sidebar-text: var(--text-100);
  --sidebar-text-muted: var(--text-400);
  --sidebar-text-hover: var(--text-200);
  --sidebar-border: var(--background-800);
}

@media (prefers-color-scheme: dark) {
  :root {
    --sidebar-bg: #334155;
    --sidebar-text: #e2e8f0;
    --sidebar-text-muted: #94a3b8;
    --sidebar-text-hover: #f1f5f9;
    --sidebar-border: #475569;
  }
}

/* ── Buttons ── */

.btn-primary {
  padding: 0.625rem 1.25rem;
  border-radius: 0.5rem;
  background: var(--accent-500);
  color: white;
  font-weight: 700;
  font-size: 0.875rem;
  border: none;
  cursor: pointer;
  transition: background 0.2s;

  &:hover:not(:disabled) {
    background: var(--accent-600);
  }

  &:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
}

.btn-secondary {
  padding: 0.625rem 1.25rem;
  border-radius: 0.5rem;
  background: var(--accent-50);
  border: 1px solid var(--accent-300);
  color: var(--accent-700);
  font-weight: 600;
  font-size: 0.875rem;
  cursor: pointer;
  transition: all 0.15s ease;

  &:hover:not(:disabled) {
    background: var(--accent-100);
  }

  &:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
}

@media (prefers-color-scheme: dark) {
  .btn-secondary {
    background: rgba(59, 130, 246, 0.15);
    border-color: rgba(59, 130, 246, 0.35);
    color: var(--accent-400);

    &:hover:not(:disabled) {
      background: rgba(59, 130, 246, 0.25);
    }
  }
}

.btn-danger {
  padding: 0.625rem 1.25rem;
  border-radius: 0.5rem;
  background: #dc2626;
  color: white;
  font-weight: 600;
  font-size: 0.875rem;
  border: none;
  cursor: pointer;
  transition: all 0.15s ease;

  &:hover:not(:disabled) {
    background: #b91c1c;
  }

  &:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
}

@media (prefers-color-scheme: dark) {
  .btn-danger {
    background: #ef4444;

    &:hover:not(:disabled) {
      background: #dc2626;
    }
  }
}

.btn-ghost {
  padding: 0.625rem 1.25rem;
  border-radius: 0.5rem;
  background: var(--background-200);
  color: var(--text-600);
  font-weight: 600;
  font-size: 0.875rem;
  border: none;
  cursor: pointer;
  transition: all 0.15s ease;

  &:hover:not(:disabled) {
    background: var(--background-300);
  }

  &:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
}

/* ── Cards ── */

.card {
  background: var(--background-300);
  color: var(--text-950);
  border-radius: 1rem;
  padding: 1.5rem;
  box-shadow: 0 4px 6px -1px rgba(0, 0, 0, 0.1);
}

@media (min-width: 1440px) {
  .card {
    padding: 2rem;
  }
}

/* ── Badges ── */

.status-badge {
  display: inline-block;
  padding: 0.125rem 0.625rem;
  border-radius: 9999px;
  font-weight: 600;
  font-size: 0.75rem;

  &.pending {
    background: var(--status-pending-bg);
    color: var(--status-pending-text);
  }

  &.running {
    background: var(--status-running-bg);
    color: var(--status-running-text);
  }

  &.completed {
    background: var(--status-completed-bg);
    color: var(--status-completed-text);
  }

  &.failed {
    background: var(--status-failed-bg);
    color: var(--status-failed-text);
  }

  &.timedout {
    background: var(--status-timedout-bg);
    color: var(--status-timedout-text);
  }
}

.state-badge {
  display: inline-block;
  padding: 0.125rem 0.625rem;
  border-radius: 9999px;
  font-weight: 600;
  font-size: 0.75rem;

  &.draft {
    background: var(--status-pending-bg);
    color: var(--status-pending-text);
  }

  &.enabled {
    background: var(--status-completed-bg);
    color: var(--status-completed-text);
  }

  &.disabled {
    background: var(--status-failed-bg);
    color: var(--status-failed-text);
  }
}

/* ── Forms ── */

.form-input {
  padding: 0.75rem 1rem;
  border-radius: 0.5rem;
  border: 1px solid var(--color-border);
  background: var(--background-50);
  font-size: 0.875rem;
  color: var(--text-950);
  transition: border-color 0.15s;

  &:focus {
    outline: none;
    border-color: var(--accent-500);
    box-shadow: 0 0 0 2px var(--accent-100);
  }

  &::placeholder {
    color: var(--text-400);
  }
}

.form-label {
  display: block;
  font-size: 0.8rem;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 0.025em;
  color: var(--text-600);
  margin-bottom: 0.4rem;
}

.info-label {
  font-size: 0.7rem;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 0.05em;
  opacity: 0.6;
}

/* ── Layout ── */

.back-link {
  display: inline-block;
  margin-bottom: 1rem;
  font-size: 0.9rem;
  color: var(--accent-500);
  text-decoration: none;

  &:hover {
    text-decoration: underline;
  }
}

.page {
  display: flex;
  flex-direction: column;
  padding: 2rem 1.5rem;
  gap: 2rem;
}

.page-header {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}

.page-title {
  margin: 0;
  font-size: 1.75rem;
  font-weight: 800;
}

@media (min-width: 1440px) {
  .page-title {
    font-size: 2rem;
  }
}

.page-subtitle {
  margin: 0;
  font-size: 0.9rem;
  color: var(--text-500);
}

.header-main {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  gap: 1rem;
}

.header-actions {
  display: flex;
  gap: 0.75rem;
}

.title-group {
  .page-title {
    margin: 0;
    font-size: 2rem;
    font-weight: 800;
  }

  .record-id {
    margin: 0.25rem 0 0;
    font-size: 0.9rem;
    font-family: monospace;
    opacity: 0.5;
  }
}

@media (min-width: 1440px) {
  .title-group .page-title {
    font-size: 2.25rem;
  }
}

.section-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 1.5rem;
  border-bottom: 1px solid rgba(0, 0, 0, 0.1);
  padding-bottom: 0.75rem;

  h2 {
    margin: 0;
    font-size: 1.25rem;
    font-weight: 700;
  }
}

/* ── Utilities ── */

.empty-state {
  padding: 2rem;
  text-align: center;
  background: var(--background-300);
  border-radius: 0.5rem;

  p {
    margin: 0;
    color: var(--text-500);
    font-size: 0.9rem;
  }
}

.state-card {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 0.75rem;
  padding: 3rem 1.5rem;
  border-radius: 1rem;
  background-color: var(--background-300);
  color: var(--text-950);
  text-align: center;

  .state-label {
    margin: 0;
    font-size: 1.1rem;
    font-weight: 700;
  }

  .state-text {
    margin: 0;
    font-size: 0.9rem;
    opacity: 0.7;
  }

  &.state-error {
    border: 1px solid var(--accent-400);

    .state-label {
      color: var(--accent-500);
    }
  }
}

.monospace {
  font-family: ui-monospace, monospace;
  font-size: 0.85rem;
  background: rgba(0, 0, 0, 0.05);
  padding: 0.2rem 0.4rem;
  border-radius: 0.25rem;
  word-break: break-all;
}

/* ── Modal ── */

.modal-overlay {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background: rgba(0, 0, 0, 0.6);
  backdrop-filter: blur(4px);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 9999;
}

.modal-content {
  background: var(--background-300);
  padding: 1.5rem;
  border-radius: 1rem;
  max-width: 400px;
  width: 100%;
  box-shadow: 0 20px 25px -5px rgba(0, 0, 0, 0.3), 0 10px 10px -5px rgba(0, 0, 0, 0.2);

  h3 {
    margin: 0 0 1rem;
    font-size: 1.25rem;
    font-weight: 700;
  }

  p {
    margin: 0 0 1.5rem;
    font-size: 0.9rem;
    color: var(--text-500);
  }
}

.modal-actions {
  display: flex;
  justify-content: flex-end;
  gap: 0.75rem;
}

/* ── Details Button ── */

.details-btn {
  background: none;
  border: none;
  cursor: pointer;
  padding: 0.5rem;
  font-size: 1.25rem;
  opacity: 0.7;
  transition: opacity 0.2s;
  color: var(--text);
  flex-shrink: 0;

  &:hover {
    opacity: 1;
  }
}

/* ── Assignment List ── */

.assignment-list {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}

.assignment-item {
  display: flex;
  align-items: center;
  gap: 0.75rem;
  padding: 0.75rem 1rem;
  background: var(--background-200);
  border-radius: 0.5rem;
  transition: background 0.2s;

  &:hover {
    background: var(--background-100);
  }
}

.assignment-badges {
  display: flex;
  gap: 0.5rem;
}

.assignment-name {
  flex: 1;
  font-weight: 500;
}

/* ── Info Grid ── */

.info-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
  gap: 1.5rem;
}

@media (min-width: 1440px) {
  .info-grid {
    grid-template-columns: repeat(auto-fit, minmax(250px, 1fr));
    gap: 2rem;
  }
}

.info-item {
  display: flex;
  flex-direction: column;
  gap: 0.25rem;

  .label {
    @extend .info-label;
  }

  .value {
    font-size: 1rem;
    width: fit-content;

    &.monospace {
      @extend .monospace;
    }
  }
}

.auth-layout {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 100%;
  height: 100vh;
  background: var(--background);
}

.loading-screen {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  height: 100vh;
  gap: 1rem;
  background: var(--background);
  color: var(--text);
}

.spinner {
  width: 2.5rem;
  height: 2.5rem;
  border: 3px solid var(--primary-300);
  border-top-color: var(--accent-500);
  border-radius: 50%;
  animation: spin 0.8s linear infinite;
}

@keyframes spin {
  to {
    transform: rotate(360deg);
  }
}

.loading-text {
  font-size: 1rem;
  opacity: 0.7;
}

/* ── Data Table ── */

.data-table {
  width: 100%;
  border-collapse: separate;
  border-spacing: 0;
  font-size: 0.875rem;

  thead {
    th {
      padding: 0.75rem 1rem;
      text-align: left;
      font-weight: 600;
      font-size: 0.75rem;
      text-transform: uppercase;
      letter-spacing: 0.05em;
      color: var(--text-500);
      background: var(--background-200);
      border-bottom: 1px solid var(--background-400);
      white-space: nowrap;
      user-select: none;

      &:first-child {
        border-radius: 0.5rem 0 0 0;
      }

      &:last-child {
        border-radius: 0 0.5rem 0 0;
      }

      &.sortable {
        cursor: pointer;

        &:hover {
          color: var(--text-700);
        }
      }
    }
  }

  tbody {
    tr {
      transition: background-color 0.15s;

      &:hover {
        background: var(--background-200);
      }

      &+tr {
        border-top: 1px solid var(--background-300);
      }

      td {
        padding: 0.75rem 1rem;
        color: var(--text-800);
        vertical-align: middle;
      }
    }
  }
}

@media (min-width: 1024px) {
  .data-table {

    thead th,
    tbody td {
      padding: 0.875rem 1.25rem;
    }
  }
}

@media (min-width: 1440px) {
  .data-table {
    font-size: 0.9rem;

    thead th,
    tbody td {
      padding: 1rem 1.5rem;
    }
  }
}

.data-table-card {
  background: var(--background-50);
  border: 1px solid var(--background-400);
  border-radius: 0.75rem;
  overflow: hidden;
  width: 100%;
}

/* ── Top Bar ── */

.top-bar {
  display: flex;
  align-items: center;
  height: 60px;
  padding: 0 1.5rem;
  background: var(--background-50);
  border-bottom: 1px solid var(--background-400);
  gap: 1rem;
  flex-shrink: 0;
}

@media (min-width: 1440px) {
  .top-bar {
    height: 68px;
    padding: 0 2rem;
  }
}

.top-bar-left {
  display: flex;
  align-items: center;
  gap: 0.75rem;
  flex: 1;
  min-width: 0;
}

.top-bar-right {
  display: flex;
  align-items: center;
  gap: 0.75rem;
  flex-shrink: 0;
}

.top-bar-title {
  font-size: 1.125rem;
  font-weight: 700;
  color: var(--text-900);
  white-space: nowrap;
}

.top-bar-breadcrumbs {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  font-size: 0.8rem;
  color: var(--text-500);

  a,
  span {
    color: var(--text-500);
    text-decoration: none;
    white-space: nowrap;

    &:hover {
      color: var(--accent-500);
    }
  }

  .separator {
    color: var(--text-400);
  }

  .current {
    color: var(--text-800);
    font-weight: 600;
  }
}

/* ── Hamburger Button ── */

.hamburger-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 36px;
  height: 36px;
  border-radius: 0.5rem;
  background: none;
  border: none;
  color: var(--text-600);
  cursor: pointer;
  transition: background 0.15s, color 0.15s;
  flex-shrink: 0;

  &:hover {
    background: var(--background-300);
    color: var(--text-900);
  }
}

/* ── Sidebar Overlay (mobile) ── */

.sidebar-overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.5);
  z-index: 999;
  display: none;
}

@media (max-width: 768px) {
  .sidebar-overlay.visible {
    display: block;
  }
}

/* ── Design Layout ── */

.design-layout {
  display: flex;
  height: 100vh;
  overflow: hidden;
  width: 100%;
}

.design-content {
  flex: 1;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  min-width: 0;
  width: 100%;
}

.design-main {
  flex: 1;
  overflow-y: auto;
  padding: 1.5rem;
  background: var(--background);
}

@media (min-width: 1024px) {
  .design-main {
    padding: 2rem;
  }
}

@media (min-width: 1440px) {
  .design-main {
    padding: 2.5rem 3rem;
  }
}

/* ── Search Input ── */

.search-input {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  padding: 0.5rem 0.75rem;
  border-radius: 0.5rem;
  background: var(--background-200);
  border: 1px solid transparent;
  transition: border-color 0.15s, background 0.15s;
  width: 200px;

  &:focus-within {
    border-color: var(--accent-400);
    background: var(--background-50);
  }

  input {
    border: none;
    background: none;
    outline: none;
    font-size: 0.8rem;
    color: var(--text-800);
    width: 100%;

    &::placeholder {
      color: var(--text-400);
    }
  }

  svg {
    flex-shrink: 0;
    color: var(--text-400);
  }
}

@media (min-width: 1440px) {
  .search-input {
    width: 280px;
    padding: 0.625rem 1rem;

    input {
      font-size: 0.875rem;
    }
  }
}

/* ── Stats Grid ── */

.stats-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(240px, 1fr));
  gap: 1rem;
}

@media (min-width: 1024px) {
  .stats-grid {
    gap: 1.5rem;
  }
}

@media (min-width: 1440px) {
  .stats-grid {
    grid-template-columns: repeat(auto-fit, minmax(280px, 1fr));
    gap: 1.5rem;
  }
}

.stat-card {
  background: var(--background-50);
  border: 1px solid var(--background-400);
  border-radius: 0.75rem;
  padding: 1.25rem;
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
  transition: box-shadow 0.15s, border-color 0.15s;

  &:hover {
    border-color: var(--accent-300);
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.05);
  }
}

@media (min-width: 1024px) {
  .stat-card {
    padding: 1.5rem;
  }
}

@media (min-width: 1440px) {
  .stat-card {
    padding: 1.75rem;
    gap: 0.75rem;
  }
}

.stat-card-header {
  display: flex;
  align-items: center;
  justify-content: space-between;

  .stat-icon {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 40px;
    height: 40px;
    border-radius: 0.5rem;
    background: var(--accent-50);
    color: var(--accent-600);
  }
}

.stat-card-label {
  font-size: 0.8rem;
  font-weight: 500;
  color: var(--text-500);
}

.stat-card-value {
  font-size: 1.75rem;
  font-weight: 800;
  color: var(--text-900);
  line-height: 1;
}

.stat-card-change {
  font-size: 0.75rem;
  font-weight: 600;

  &.up {
    color: var(--status-completed-text);
  }

  &.down {
    color: var(--status-failed-text);
  }
}

/* ── View Toggle ── */

.view-toggle {
  display: flex;
  border: 1px solid var(--background-400);
  border-radius: 0.5rem;
  overflow: hidden;

  button {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 36px;
    height: 36px;
    background: none;
    border: none;
    color: var(--text-500);
    cursor: pointer;
    transition: background 0.15s, color 0.15s;

    &:hover {
      background: var(--background-200);
    }

    &.active {
      background: var(--accent-50);
      color: var(--accent-600);
    }

    &+button {
      border-left: 1px solid var(--background-400);
    }
  }
}

/* ── Section ── */

.section-header-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 1rem;
  gap: 1rem;

  h2 {
    font-size: 1.125rem;
    font-weight: 700;
    color: var(--text-900);
    margin: 0;
  }
}

.section-actions {
  display: flex;
  align-items: center;
  gap: 0.5rem;
}

/* ── Empty State ── */

.empty-state-icon {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 48px;
  height: 48px;
  border-radius: 50%;
  background: var(--background-300);
  color: var(--text-400);
  margin-bottom: 0.75rem;
}

.empty-state-block {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 3rem 1.5rem;
  text-align: center;
  color: var(--text-500);

  p {
    margin: 0;
    font-size: 0.9rem;
  }

  .empty-title {
    font-weight: 600;
    color: var(--text-700);
    margin-bottom: 0.25rem;
  }
}

.error-icon {
  font-size: 3rem;
}

.error-title {
  font-size: 1.25rem;
  font-weight: 700;
}

.error-detail {
  font-size: 0.9rem;
  opacity: 0.7;
}
</style>
