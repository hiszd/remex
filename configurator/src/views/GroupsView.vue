<script setup lang="ts">
import { ref } from "vue"
import { getGroups } from "@/lib/api"
import { useQuery } from "@tanstack/vue-query"
import { useSurrealClient } from "@/lib/surreal"
import QueryState from "@/components/QueryState.vue"

const client = useSurrealClient()

const { data: groups, isLoading, isError, error } = useQuery({
  queryKey: ["groups"],
  queryFn: () => getGroups(client),
  select: (data) => Object.freeze(data),
})

const viewMode = ref<"table" | "grid">("table")
</script>

<template>
  <div class="page">
    <div class="section-header-row">
      <div>
        <p v-if="groups && groups.length > 0" class="page-subtitle">
          {{ groups.length }} {{ groups.length === 1 ? "group" : "groups" }}
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
        <router-link to="/groups/new" class="btn-primary">Create Group</router-link>
      </div>
    </div>

    <QueryState
      :is-loading="isLoading"
      :is-error="isError"
      :error="error"
      :is-empty="!groups || groups.length === 0"
      empty-label="No groups yet"
      empty-subtext="Groups will appear here once they are created."
    >
      <div v-if="viewMode === 'table'" class="data-table-card">
        <table class="data-table">
          <thead>
            <tr>
              <th>Group Name</th>
              <th>Members</th>
              <th>Created</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="group in groups" :key="String(group.id)" @click="$router.push(`/groups/${group.id}`)" style="cursor: pointer;">
              <td>
                <span style="font-weight: 600; color: var(--text-900);">{{ group.group_name }}</span>
              </td>
              <td>
                <span style="color: var(--text-600); font-size: 0.85rem;">{{ group.members?.length ?? 0 }} members</span>
              </td>
              <td>
                <span style="color: var(--text-500); font-size: 0.85rem;">{{ group.created_at ? new Date(group.created_at).toLocaleDateString() : "-" }}</span>
              </td>
            </tr>
          </tbody>
        </table>
      </div>

      <div v-else class="stats-grid">
        <div v-for="group in groups" :key="String(group.id)" class="stat-card" @click="$router.push(`/groups/${group.id}`)" style="cursor: pointer;">
          <span class="stat-card-value" style="font-size: 1.25rem;">{{ group.group_name }}</span>
          <div style="display: flex; justify-content: space-between; align-items: center; margin-top: 0.5rem;">
            <span style="color: var(--text-500); font-size: 0.85rem;">{{ group.members?.length ?? 0 }} members</span>
            <span style="color: var(--text-500); font-size: 0.8rem;">{{ group.created_at ? new Date(group.created_at).toLocaleDateString() : "" }}</span>
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
