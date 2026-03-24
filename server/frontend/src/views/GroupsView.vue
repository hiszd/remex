<script setup lang="ts">
import { computed } from 'vue';
import { getGroups, getGroupClients, type Group, type ClientSrv } from '@/client/index';
import { useQuery, useQueries } from '@tanstack/vue-query';
import GroupComponent from '@/components/Group.vue';

const { data: groups, isLoading, isError, error } = useQuery({
  queryKey: ['groups'],
  queryFn: async () => {
    const { data, error } = await getGroups();
    if (error) throw error;
    return data as Group[];
  },
});

const clientQueries = useQueries({
  queries: (groups.value ?? []).map((group: Group) => ({
    queryKey: ['groups', group.id, 'clients'],
    queryFn: async () => {
      const { data, error } = await getGroupClients({ path: { group_id: group.id } });
      if (error) throw error;
      return { groupId: group.id, clients: (data as ClientSrv[]).slice(0, 5) };
    },
    enabled: group.client_count > 0,
  })),
});

const groupClientsMap = computed(() => {
  const map = new Map<string, ClientSrv[]>();
  const queries = clientQueries.value;
  for (const query of queries) {
    if (query.data) {
      map.set(query.data.groupId, query.data.clients);
    }
  }
  return map;
});
</script>

<template>
  <div class="page">
    <header class="page-header">
      <div class="header-top">
        <h1 class="page-title">Groups</h1>
        <router-link to="/groups/new" class="btn-primary">Create Group</router-link>
      </div>
      <p class="page-subtitle" v-if="groups && groups.length > 0">
        {{ groups.length }} {{ groups.length === 1 ? 'group' : 'groups' }}
      </p>
    </header>

    <div v-if="isLoading" class="state-card">
      <div class="spinner" />
      <p class="state-text">Fetching groups from backend...</p>
    </div>

    <div v-else-if="isError" class="state-card state-error">
      <p class="state-label">Something went wrong</p>
      <p class="state-text">{{ error }}</p>
    </div>

    <div v-else-if="!groups || groups.length === 0" class="state-card state-empty">
      <p class="state-label">No groups yet</p>
      <p class="state-text">Groups will appear here once they are created.</p>
    </div>

    <div v-else class="card-grid">
      <GroupComponent
        v-for="group in groups"
        :group="group"
        :clients="groupClientsMap.get(group.id)"
        :key="group.id"
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
  .header-top {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

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
  to {
    transform: rotate(360deg);
  }
}

.btn-primary {
  padding: 0.6rem 1.25rem;
  border-radius: 0.5rem;
  background-color: var(--accent-500);
  color: white;
  font-weight: 700;
  text-decoration: none;
  font-size: 0.9rem;
  transition: background 0.2s;

  &:hover {
    background-color: var(--accent-600);
  }
}
</style>
