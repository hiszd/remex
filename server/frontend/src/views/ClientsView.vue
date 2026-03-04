<script setup lang="ts">
import { getClients } from '@/client/index'; // Generated from Rust!
import { useQuery } from '@tanstack/vue-query';
import Client from '@/components/Client.vue';

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
  <div class="container">
    <h1 class="client-header">Client List</h1>
    <div class="client-container">

      <div v-if="isLoading">Fetching clients from Rust backend...</div>

      <div v-else-if="isError">Error: {{ error }}</div>

      <div v-else>
        <Client v-for="client in clients" :client="client" :key="client.id" />
      </div>

      <div v-if="clients?.length === 0">No clients found in database.</div>
    </div>
  </div>
</template>

<style lang="scss" scoped>
.client-container {
  display: flex;
  flex-direction: column;
  align-items: center;
}

.container {
  display: flex;
  flex-direction: column;
  align-items: center;
}

.client-header {
  display: block;
  text-align: center;
  padding: 1rem;
}
</style>
