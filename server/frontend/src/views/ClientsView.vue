<script setup lang="ts">
import { getClients } from '@/client/index'; // Generated from Rust!
import { useQuery } from '@tanstack/vue-query';

// We wrap the API call in a query
const { data: clients, isLoading, isError, error } = useQuery({
  queryKey: ['clients'],
  queryFn: async () => {
    const { data, error } = await getClients();
    if (error) throw error;
    return data;
  },
});
</script>

<template>
  <div class="client-container">
    <h2>Client List</h2>

    <div v-if="isLoading">Fetching clients from Rust backend...</div>

    <div v-else-if="isError">Error: {{ error }}</div>

    <ul v-else>
      <li v-for="client in clients" :key="client.id">
        {{ client.id }} — <small>{{ client.client_name }}</small>
      </li>
    </ul>

    <div v-if="clients?.length === 0">No clients found in database.</div>
  </div>
</template>
