<script setup lang="ts">
import { ref, computed } from "vue"
import { getClients, getJobs, getGroups, getJobsForClient, getGroupsForClient } from "@/lib/api"
import { useQuery } from "@tanstack/vue-query"
import { useSurrealClient } from "@/lib/surreal"
import QueryState from "@/components/QueryState.vue"

const client = useSurrealClient()

const { data: clients, isLoading, isError, error } = useQuery({
  queryKey: ["clients"],
  queryFn: () => getClients(client),
  select: (data) => Object.freeze(data),
})

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

const clientJobCounts = computed(() => {
  if (!clients.value || !jobs.value || !groups.value) return new Map()
  const map = new Map<string, number>()
  for (const c of clients.value) {
    map.set(String(c.id), getJobsForClient(c.id, jobs.value, groups.value).length)
  }
  return map
})

const clientGroupCounts = computed(() => {
  if (!clients.value || !groups.value) return new Map()
  const map = new Map<string, number>()
  for (const c of clients.value) {
    map.set(String(c.id), getGroupsForClient(c.id, groups.value).length)
  }
  return map
})

const viewMode = ref<"table" | "grid">("table")
</script>

<template>
  <div class="page">
    <div class="section-header-row">
      <div>
        <p v-if="clients && clients.length > 0" class="page-subtitle">
          {{ clients.length }} registered {{ clients.length === 1 ? "client" : "clients" }}
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
      </div>
    </div>

    <QueryState
      :is-loading="isLoading"
      :is-error="isError"
      :error="error"
      :is-empty="!clients || clients.length === 0"
      empty-label="No clients yet"
      empty-subtext="Clients will appear here once they connect."
    >
      <div v-if="viewMode === 'table'" class="data-table-card">
        <table class="data-table">
          <thead>
            <tr>
              <th>Client Name</th>
              <th>Hardware Hash</th>
              <th>Jobs</th>
              <th>Groups</th>
              <th>Last Seen</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="c in clients" :key="String(c.id)" @click="$router.push(`/clients/${c.id}`)" style="cursor: pointer;">
              <td>
                <span style="font-weight: 600; color: var(--text-900);">{{ c.client_name }}</span>
              </td>
              <td>
                <span class="monospace" style="font-size: 0.8rem;">{{ c.hardware_hash?.slice(0, 16) }}...</span>
              </td>
              <td>
                <span style="color: var(--text-600); font-size: 0.85rem;">{{ clientJobCounts.get(String(c.id)) ?? 0 }}</span>
              </td>
              <td>
                <span style="color: var(--text-600); font-size: 0.85rem;">{{ clientGroupCounts.get(String(c.id)) ?? 0 }}</span>
              </td>
              <td>
                <span style="color: var(--text-500); font-size: 0.85rem;">{{ c.last_seen ? new Date(c.last_seen).toLocaleDateString() : "Never" }}</span>
              </td>
            </tr>
          </tbody>
        </table>
      </div>

      <div v-else class="stats-grid">
        <div v-for="c in clients" :key="String(c.id)" class="stat-card" @click="$router.push(`/clients/${c.id}`)" style="cursor: pointer;">
          <span class="stat-card-value" style="font-size: 1.25rem;">{{ c.client_name }}</span>
          <div style="display: flex; justify-content: space-between; align-items: center; margin-top: 0.5rem;">
            <span style="color: var(--text-500); font-size: 0.85rem;">{{ clientJobCounts.get(String(c.id)) ?? 0 }} jobs</span>
            <span style="color: var(--text-500); font-size: 0.85rem;">{{ clientGroupCounts.get(String(c.id)) ?? 0 }} groups</span>
          </div>
          <span style="color: var(--text-400); font-size: 0.75rem; margin-top: 0.25rem;">{{ c.last_seen ? new Date(c.last_seen).toLocaleDateString() : "Never" }}</span>
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
