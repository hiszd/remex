<script setup lang="ts">
import { getClients, getJobs, getGroups, getJobsForClient, getGroupsForClient } from "@/lib/api"
import { useQuery } from "@tanstack/vue-query"
import { useSurrealClient } from "@/lib/surreal"
import Client1 from "@/components/Client1.vue"

const client = useSurrealClient()

const { data: clients, isLoading, isError, error } = useQuery({
  queryKey: ["clients"],
  queryFn: () => getClients(client),
})

const { data: jobs } = useQuery({
  queryKey: ["jobs"],
  queryFn: () => getJobs(client),
})

const { data: groups } = useQuery({
  queryKey: ["groups"],
  queryFn: () => getGroups(client),
})
</script>

<template>
  <div class="page">
    <header class="page-header">
      <h1 class="page-title">Clients</h1>
      <p class="page-subtitle" v-if="clients && clients.length > 0">
        {{ clients.length }} registered {{ clients.length === 1 ? "client" : "clients" }}
      </p>
    </header>

    <div v-if="isLoading" class="state-card">
      <div class="spinner" />
      <p class="state-text">Loading clients...</p>
    </div>

    <div v-else-if="isError" class="state-card state-error">
      <p class="state-label">Something went wrong</p>
      <p class="state-text">{{ error }}</p>
    </div>

    <div v-else-if="!clients || clients.length === 0" class="state-card state-empty">
      <p class="state-label">No clients yet</p>
      <p class="state-text">Clients will appear here once they connect.</p>
    </div>

    <div v-else class="card-grid">
      <Client1
        v-for="c in clients"
        :client="c"
        :key="String(c.id)"
        :job-count="jobs && groups ? getJobsForClient(c.id, jobs, groups).length : 0"
        :group-count="groups ? getGroupsForClient(c.id, groups).length : 0"
      />
    </div>
  </div>
</template>

<style lang="scss" scoped>
.page {
  width: 100%;
  max-width: 1400px;
  margin: 0 auto;
}

.card-grid {
  display: flex;
  flex-direction: column;
  gap: 1.5rem;
  width: 100%;
}
</style>
