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
  select: (data) => Object.freeze(data),
})

const { data: groups } = useQuery({
  queryKey: ["groups"],
  queryFn: () => getGroups(client),
  select: (data) => Object.freeze(data),
})

const { data: clients } = useQuery({
  queryKey: ["clients"],
  queryFn: () => getClients(client),
  select: (data) => Object.freeze(data),
  refetchInterval: 30_000,
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

const onlineCount = computed(() => {
  if (!clients.value) return "-"
  const cutoff = Date.now() - 60_000
  return clients.value.filter((c) => {
    if (!c.last_seen) return false
    return new Date(c.last_seen).getTime() > cutoff
  }).length
})

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
    <div class="stats-grid">
      <router-link to="/jobs" class="stat-card">
        <div class="stat-card-header">
          <span class="stat-card-label">Total Jobs</span>
          <span class="stat-icon">
            <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
              <rect x="2" y="7" width="20" height="14" rx="2" ry="2" />
              <path d="M16 21V5a2 2 0 00-2-2h-4a2 2 0 00-2 2v16" />
            </svg>
          </span>
        </div>
        <span class="stat-card-value">{{ jobs?.length ?? "-" }}</span>
      </router-link>

      <router-link to="/groups" class="stat-card">
        <div class="stat-card-header">
          <span class="stat-card-label">Groups</span>
          <span class="stat-icon">
            <svg width="20" height="20" viewBox="0 0 20 20" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
              <circle cx="7" cy="7" r="3" />
              <path d="M2 17v-1a4 4 0 014-4h2a4 4 0 014 4v1" />
              <circle cx="14" cy="8" r="2" />
              <path d="M18 17v-1a3 3 0 00-3-3h-1" />
            </svg>
          </span>
        </div>
        <span class="stat-card-value">{{ groups?.length ?? "-" }}</span>
      </router-link>

      <router-link to="/clients" class="stat-card">
        <div class="stat-card-header">
          <span class="stat-card-label">Clients</span>
          <span class="stat-icon">
            <svg width="20" height="20" viewBox="0 0 20 20" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
              <rect x="3" y="4" width="14" height="10" rx="1" />
              <path d="M8 17h4M10 14v3" />
            </svg>
          </span>
        </div>
        <span class="stat-card-value">{{ clients?.length ?? "-" }}</span>
      </router-link>

      <router-link to="/clients" class="stat-card">
        <div class="stat-card-header">
          <span class="stat-card-label">Online Endpoints</span>
          <span class="stat-icon">
            <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
              <path d="M13 2L3 14h9l-1 8 10-12h-9l1-8z" />
            </svg>
          </span>
        </div>
        <span class="stat-card-value">{{ onlineCount }}</span>
      </router-link>
    </div>

    <div>
      <div class="section-header-row">
        <h2>Recent Jobs</h2>
        <router-link to="/jobs" class="btn-ghost">View All</router-link>
      </div>

      <div v-if="!jobs" class="state-card">
        <div class="spinner" />
        <p class="state-text">Loading...</p>
      </div>

      <div v-else-if="recentJobs.length === 0" class="empty-state-block">
        <div class="empty-state-icon">
          <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
            <rect x="2" y="7" width="20" height="14" rx="2" ry="2" />
            <path d="M16 21V5a2 2 0 00-2-2h-4a2 2 0 00-2 2v16" />
          </svg>
        </div>
        <p class="empty-title">No jobs yet</p>
        <p class="empty-text">Create a job to get started.</p>
        <router-link to="/jobs/new" class="btn-primary" style="margin-top: 1rem;">Create Job</router-link>
      </div>

      <div v-else class="data-table-card">
        <table class="data-table">
          <thead>
            <tr>
              <th>Job Name</th>
              <th>Status</th>
              <th>Created</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="job in recentJobs" :key="String(job.id)" @click="$router.push(`/jobs/${job.id}`)" style="cursor: pointer;">
              <td>
                <span style="font-weight: 600; color: var(--text-900);">{{ job.job_name }}</span>
              </td>
              <td>
                <span class="status-badge" :class="extractEnumVariant(job.execution_status).toLowerCase()">
                  {{ formatEnumVariant(extractEnumVariant(job.execution_status)) }}
                </span>
              </td>
              <td>
                <span style="color: var(--text-500); font-size: 0.85rem;">{{ formatTimeAgo(job.created_at) }}</span>
              </td>
            </tr>
          </tbody>
        </table>
      </div>
    </div>
  </div>
</template>

<style lang="scss" scoped>
</style>
