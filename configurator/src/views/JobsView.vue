<script setup lang="ts">
import { ref, computed } from "vue"
import { getJobs } from "@/lib/api"
import { useQuery } from "@tanstack/vue-query"
import { useSurrealClient } from "@/lib/surreal"
import { extractEnumVariant, formatEnumVariant } from "@/lib/model"
import QueryState from "@/components/QueryState.vue"

const client = useSurrealClient()

const { data: jobs, isLoading, isError, error } = useQuery({
  queryKey: ["jobs"],
  queryFn: () => getJobs(client),
  select: (data) => Object.freeze(data),
})

const viewMode = ref<"table" | "grid">("table")
</script>

<template>
  <div class="page">
    <div class="section-header-row">
      <div>
        <p v-if="jobs && jobs.length > 0" class="page-subtitle">
          {{ jobs.length }} {{ jobs.length === 1 ? "job" : "jobs" }} configured
        </p>
      </div>
      <div class="section-actions">
        <div class="view-toggle">
          <button :class="{ active: viewMode === 'table' }" @click="viewMode = 'table'" title="Table view">
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
              <rect x="3" y="3" width="18" height="18" rx="2" />
              <path d="M3 9h18M3 15h18M9 3v18M15 3v18" />
            </svg>
          </button>
          <button :class="{ active: viewMode === 'grid' }" @click="viewMode = 'grid'" title="Grid view">
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
              <rect x="3" y="3" width="7" height="7" />
              <rect x="14" y="3" width="7" height="7" />
              <rect x="3" y="14" width="7" height="7" />
              <rect x="14" y="14" width="7" height="7" />
            </svg>
          </button>
        </div>
        <router-link to="/jobs/new" class="btn-primary">Create Job</router-link>
      </div>
    </div>

    <QueryState
      :is-loading="isLoading"
      :is-error="isError"
      :error="error"
      :is-empty="!jobs || jobs.length === 0"
      empty-label="No jobs found"
      empty-subtext="Jobs will appear here once they are created."
    >
      <div v-if="viewMode === 'table'" class="data-table-card">
        <table class="data-table">
          <thead>
            <tr>
              <th>Job Name</th>
              <th>Type</th>
              <th>State</th>
              <th>Status</th>
              <th>Created</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="job in jobs" :key="String(job.id)" @click="$router.push(`/jobs/${job.id}`)" style="cursor: pointer;">
              <td>
                <span style="font-weight: 600; color: var(--text-900);">{{ job.job_name }}</span>
              </td>
              <td>
                <span style="color: var(--text-600); font-size: 0.85rem;">{{ extractEnumVariant(job.job_type) }}</span>
              </td>
              <td>
                <span class="state-badge" :class="extractEnumVariant(job.enabled).toLowerCase()">
                  {{ extractEnumVariant(job.enabled) }}
                </span>
              </td>
              <td>
                <span class="status-badge" :class="extractEnumVariant(job.execution_status).toLowerCase()">
                  {{ formatEnumVariant(extractEnumVariant(job.execution_status)) }}
                </span>
              </td>
              <td>
                <span style="color: var(--text-500); font-size: 0.85rem;">{{ job.created_at ? new Date(job.created_at).toLocaleDateString() : "-" }}</span>
              </td>
            </tr>
          </tbody>
        </table>
      </div>

      <div v-else class="stats-grid">
        <div v-for="job in jobs" :key="String(job.id)" class="stat-card" @click="$router.push(`/jobs/${job.id}`)" style="cursor: pointer;">
          <div class="stat-card-header">
            <span class="stat-card-label">{{ extractEnumVariant(job.job_type) }}</span>
            <span class="status-badge" :class="extractEnumVariant(job.execution_status).toLowerCase()">
              {{ formatEnumVariant(extractEnumVariant(job.execution_status)) }}
            </span>
          </div>
          <span class="stat-card-value" style="font-size: 1.25rem;">{{ job.job_name }}</span>
          <div style="display: flex; justify-content: space-between; align-items: center; margin-top: 0.5rem;">
            <span class="state-badge" :class="extractEnumVariant(job.enabled).toLowerCase()">
              {{ extractEnumVariant(job.enabled) }}
            </span>
            <span style="color: var(--text-500); font-size: 0.8rem;">{{ job.created_at ? new Date(job.created_at).toLocaleDateString() : "" }}</span>
          </div>
        </div>
      </div>
    </QueryState>
  </div>
</template>

<style lang="scss" scoped>
.stats-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(300px, 1fr));
  gap: 1rem;
}
</style>
