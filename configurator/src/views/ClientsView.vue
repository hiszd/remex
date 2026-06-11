<script setup lang="ts">
import { getClients, getJobs, getGroups, getJobsForClient, getGroupsForClient } from "@/lib/api"
import { useQuery } from "@tanstack/vue-query"
import { useSurrealClient } from "@/lib/surreal"
import Client1 from "@/components/Client1.vue"
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
</script>

<template>
  <div class="page">
    <header class="page-header">
      <h1 class="page-title">Clients</h1>
      <p class="page-subtitle" v-if="clients && clients.length > 0">
        {{ clients.length }} registered {{ clients.length === 1 ? "client" : "clients" }}
      </p>
    </header>

    <QueryState
      :is-loading="isLoading"
      :is-error="isError"
      :error="error"
      :is-empty="!clients || clients.length === 0"
      empty-label="No clients yet"
      empty-subtext="Clients will appear here once they connect."
    >
      <div class="card-grid">
        <Client1
          v-for="c in clients"
          :client="c"
          :key="String(c.id)"
          :job-count="jobs && groups ? getJobsForClient(c.id, jobs, groups).length : 0"
          :group-count="groups ? getGroupsForClient(c.id, groups).length : 0"
        />
      </div>
    </QueryState>
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
