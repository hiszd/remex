<script setup lang="ts">
import { getClients } from "@/lib/api"
import { useQuery } from "@tanstack/vue-query"
import { useSurrealClient } from "@/lib/surreal"
import Client1 from "@/components/Client1.vue"

const client = useSurrealClient()

const { data: clients, isLoading, isError, error } = useQuery({
  queryKey: ["clients"],
  queryFn: () => getClients(client),
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
      />
    </div>
  </div>
</template>

<style lang="scss" scoped>
.page {
  display: flex;
  flex-direction: column;
  width: 100%;
  max-width: 1400px;
  margin: 0 auto;
  padding: 2rem 1.5rem;
  gap: 1.5rem;
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

.card-grid {
  display: flex;
  flex-direction: column;
  gap: 1.5rem;
  width: 100%;
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

.state-error {
  border: 1px solid var(--accent-400);

  .state-label {
    color: var(--accent-500);
  }
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
  to { transform: rotate(360deg); }
}
</style>
