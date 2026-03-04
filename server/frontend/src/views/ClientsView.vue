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

    <div v-else>
      <ul v-for="client in clients" :key="client.id">
        <span>{{ client.client_name }}</span><br />
        <small>ID: </small><span>{{ client.id }}</span><br />
        <small>Secret: </small><span>{{ client.secret }}</span><br />
        <small>Hash: </small><span>{{ client.hardware_hash }}</span><br />
        <small>Created: </small><span>{{ client.created_at }}</span><br />
        <small>Updated: </small><span>{{ client.updated_at }}</span>
      </ul>
    </div>

    <div v-if="clients?.length === 0">No clients found in database.</div>
  </div>
</template>
