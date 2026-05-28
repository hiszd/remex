<script setup lang="ts">
import { computed } from "vue"
import { getJobs, getGroups, getClients } from "@/lib/api"
import { useQuery } from "@tanstack/vue-query"
import { useSurrealClient } from "@/lib/surreal"
import { extractEnumVariant, formatEnumVariant } from "@/lib/model"
import { formatDistanceToNow } from "@/utils/date"

const client = useSurrealClient()

const { data: jobs } = useQuery({
  queryKey: ["jobs"],
  queryFn: () => getJobs(client),
})

const { data: groups } = useQuery({
  queryKey: ["groups"],
  queryFn: () => getGroups(client),
})

const { data: clients } = useQuery({
  queryKey: ["clients"],
  queryFn: () => getClients(client),
})

const recentJobs = computed(() => {
  if (!jobs.value) return []
  return [...jobs.value]
    .sort(
      (a, b) =>
        new Date(b.created_at).getTime() - new Date(a.created_at).getTime()
    )
    .slice(0, 5)
})

function getStatusClass(status: string): string {
  switch (status.toLowerCase()) {
    case "running":
      return "status-running"
    case "completed":
      return "status-completed"
    case "failed":
      return "status-failed"
    case "timedout":
      return "status-timedout"
    default:
      return "status-pending"
  }
}

function formatTimeAgo(dateStr: string) {
  try {
    return formatDistanceToNow(new Date(dateStr))
  } catch {
    return "Unknown"
  }
}
</script>

<template>
  <div class="page">
    <header class="page-header">
      <h1 class="page-title">Dashboard</h1>
      <p class="page-subtitle">Overview of your remex system</p>
    </header>

    <div class="stats-grid">
      <router-link to="/jobs" class="stat-card">
        <div class="stat-icon jobs-icon">
          <svg
            xmlns="http://www.w3.org/2000/svg"
            width="28"
            height="28"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
            stroke-linecap="round"
            stroke-linejoin="round"
          >
            <rect width="20" height="14" x="2" y="7" rx="2" ry="2" />
            <path d="M16 21V5a2 2 0 0 0-2-2h-4a2 2 0 0 0-2 2v16" />
          </svg>
        </div>
        <div class="stat-content">
          <span class="stat-value">{{ jobs?.length ?? "-" }}</span>
          <span class="stat-label">Jobs</span>
        </div>
      </router-link>

      <router-link to="/groups" class="stat-card">
        <div class="stat-icon groups-icon">
          <svg
            xmlns="http://www.w3.org/2000/svg"
            width="28"
            height="28"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
            stroke-linecap="round"
            stroke-linejoin="round"
          >
            <path d="M16 21v-2a4 4 0 0 0-4-4H6a4 4 0 0 0-4 4v2" />
            <circle cx="9" cy="7" r="4" />
            <path d="M22 21v-2a4 4 0 0 0-3-3.87" />
            <path d="M16 3.13a4 4 0 0 1 0 7.75" />
          </svg>
        </div>
        <div class="stat-content">
          <span class="stat-value">{{ groups?.length ?? "-" }}</span>
          <span class="stat-label">Groups</span>
        </div>
      </router-link>

      <router-link to="/clients" class="stat-card">
        <div class="stat-icon clients-icon">
          <svg
            xmlns="http://www.w3.org/2000/svg"
            width="28"
            height="28"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
            stroke-linecap="round"
            stroke-linejoin="round"
          >
            <rect width="14" height="8" x="5" y="2" rx="2" />
            <rect width="20" height="8" x="2" y="14" rx="2" />
            <path d="M6 18h2" />
            <path d="M12 18h6" />
          </svg>
        </div>
        <div class="stat-content">
          <span class="stat-value">{{ clients?.length ?? "-" }}</span>
          <span class="stat-label">Clients</span>
        </div>
      </router-link>
    </div>

    <section class="recent-section">
      <div class="section-header">
        <h2 class="section-title">Recent Jobs</h2>
        <router-link to="/jobs" class="view-all-link">View all</router-link>
      </div>

      <div v-if="!jobs" class="state-card">
        <div class="spinner" />
        <p class="state-text">Loading...</p>
      </div>

      <div v-else-if="recentJobs.length === 0" class="state-card state-empty">
        <p class="state-label">No jobs yet</p>
        <p class="state-text">Create a job to get started.</p>
        <router-link to="/jobs/new" class="btn-primary">Create Job</router-link>
      </div>

      <div v-else class="recent-list">
        <router-link
          v-for="job in recentJobs"
          :key="String(job.id)"
          :to="`/jobs/${job.id}`"
          class="recent-item"
        >
          <div class="recent-item-info">
            <span class="recent-item-name">{{ job.job_name }}</span>
            <span class="recent-item-time">{{ formatTimeAgo(job.created_at) }}</span>
          </div>
          <span
            class="recent-item-status"
            :class="getStatusClass(extractEnumVariant(job.execution_status))"
          >
            {{ formatEnumVariant(extractEnumVariant(job.execution_status)) }}
          </span>
        </router-link>
      </div>
    </section>
  </div>
</template>

<style lang="scss" scoped>
.page {
  display: flex;
  flex-direction: column;
  width: 100%;
  max-width: 1000px;
  margin: 0 auto;
  padding: 2rem 1.5rem;
  gap: 2rem;
}

.page-header {
  .page-title {
    margin: 0;
    font-size: 1.75rem;
    font-weight: 800;
    color: var(--text);
  }

  .page-subtitle {
    margin: 0.25rem 0 0;
    font-size: 0.9rem;
    color: var(--text-500);
  }
}

.stats-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
  gap: 1rem;
}

.stat-card {
  display: flex;
  align-items: center;
  gap: 1rem;
  padding: 1.25rem;
  border-radius: 0.75rem;
  background-color: var(--primary-800);
  text-decoration: none;
  transition: transform 0.15s ease, box-shadow 0.15s ease;

  &:hover {
    transform: translateY(-2px);
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.15);
  }
}

.stat-icon {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 56px;
  height: 56px;
  border-radius: 0.75rem;
  color: white;

  &.jobs-icon {
    background-color: var(--accent-600);
  }

  &.groups-icon {
    background-color: var(--secondary-600);
  }

  &.clients-icon {
    background-color: var(--primary-500);
  }
}

.stat-content {
  display: flex;
  flex-direction: column;
}

.stat-value {
  font-size: 1.75rem;
  font-weight: 800;
  color: var(--text-50);
  line-height: 1;
}

.stat-label {
  font-size: 0.875rem;
  color: var(--text-400);
  margin-top: 0.25rem;
}

.section-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 1rem;
}

.section-title {
  margin: 0;
  font-size: 1.25rem;
  font-weight: 700;
  color: var(--text);
}

.view-all-link {
  font-size: 0.875rem;
  color: var(--accent-500);
  text-decoration: none;

  &:hover {
    text-decoration: underline;
  }
}

.recent-list {
  display: flex;
  flex-direction: column;
  border-radius: 0.75rem;
  overflow: hidden;
  background-color: var(--background-300);
}

.recent-item {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 1rem 1.25rem;
  text-decoration: none;
  transition: background-color 0.15s ease;

  &:not(:last-child) {
    border-bottom: 1px solid var(--background-400);
  }

  &:hover {
    background-color: var(--background-400);
  }
}

.recent-item-info {
  display: flex;
  flex-direction: column;
  gap: 0.25rem;
}

.recent-item-name {
  font-weight: 600;
  color: var(--text);
}

.recent-item-time {
  font-size: 0.8rem;
  color: var(--text-500);
}

.recent-item-status {
  padding: 0.25rem 0.75rem;
  border-radius: 1rem;
  font-size: 0.75rem;
  font-weight: 600;
  text-transform: capitalize;

  &.status-running {
    background-color: var(--status-running-bg);
    color: var(--status-running-text);
  }

  &.status-completed {
    background-color: var(--status-completed-bg);
    color: var(--status-completed-text);
  }

  &.status-failed {
    background-color: var(--status-failed-bg);
    color: var(--status-failed-text);
  }

  &.status-pending {
    background-color: var(--status-pending-bg);
    color: var(--status-pending-text);
  }

  &.status-timedout {
    background-color: var(--status-timedout-bg);
    color: var(--status-timedout-text);
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
}

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

.state-empty {
  border: 1px dashed var(--primary-400);
}

.spinner {
  width: 2rem;
  height: 2rem;
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

.btn-primary {
  margin-top: 0.5rem;
  padding: 0.6rem 1.25rem;
  border-radius: 0.5rem;
  background-color: var(--accent-500);
  color: white;
  font-weight: 700;
  text-decoration: none;
  font-size: 0.9rem;
  transition: background 0.2s;

  &:hover {
    background-color: var(--accent-600);
  }
}
</style>
