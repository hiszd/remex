<script setup lang="ts">
import { ref, watch } from 'vue';
import { getJobClientStatuses, type ClientJobStatusResponse } from '@/client/index';
import { useQuery } from '@tanstack/vue-query';

interface GroupWithClients {
  id: string;
  group_name: string;
  clients: Array<{
    id: string;
    client_name: string;
  }>;
}

const props = defineProps<{
  groups: GroupWithClients[];
  jobId: string;
}>();

const expandedGroups = ref<Set<string>>(new Set(props.groups.map(g => g.id)));

const { data: clientStatuses, isLoading: loadingStatuses } = useQuery({
  queryKey: ['job-client-statuses', props.jobId],
  queryFn: async () => {
    const { data, error } = await getJobClientStatuses({ path: { job_id: props.jobId } });
    if (error) throw error;
    return data as ClientJobStatusResponse[];
  },
  enabled: !!props.jobId,
});

const getClientStatus = (clientId: string): string => {
  if (!clientStatuses.value) return 'pending';
  const status = clientStatuses.value.find(s => s.client_id === clientId);
  return status?.status || 'pending';
};

const toggleGroup = (groupId: string) => {
  if (expandedGroups.value.has(groupId)) {
    expandedGroups.value.delete(groupId);
  } else {
    expandedGroups.value.add(groupId);
  }
};

const isExpanded = (groupId: string) => expandedGroups.value.has(groupId);
</script>

<template>
  <div class="client-tree">
    <div v-if="loadingStatuses" class="tree-loading">
      <div class="spinner-small"></div>
      <span>Loading client statuses...</span>
    </div>

    <div v-else-if="!groups || groups.length === 0" class="tree-empty">
      No groups assigned to this job.
    </div>

    <div v-else class="tree-groups">
      <div v-for="group in groups" :key="group.id" class="tree-group">
        <div class="group-header" @click="toggleGroup(group.id)">
          <span class="expand-icon" :class="{ expanded: isExpanded(group.id) }">
            <svg width="12" height="12" viewBox="0 0 12 12" fill="currentColor">
              <path d="M4 2L8 6L4 10" stroke="currentColor" stroke-width="2" fill="none" />
            </svg>
          </span>
          <span class="group-name">{{ group.group_name }}</span>
          <span class="client-count">({{ group.clients.length }} {{ group.clients.length === 1 ? 'client' : 'clients'
          }})</span>
        </div>

        <div v-if="isExpanded(group.id)" class="group-clients">
          <router-link v-for="client in group.clients" :key="client.id" :to="'/clients/' + client.id"
            class="client-row">
            <span class="client-name">{{ client.client_name }}</span>
            <span class="status-badge" :class="getClientStatus(client.id)">
              {{ getClientStatus(client.id) }}
            </span>
          </router-link>
        </div>
      </div>
    </div>
  </div>
</template>

<style lang="scss" scoped>
.client-tree {
  width: 100%;
}

.tree-loading {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  padding: 1rem;
  color: var(--text-500);
  font-size: 0.9rem;
}

.tree-empty {
  padding: 1rem;
  color: var(--text-500);
  font-size: 0.9rem;
  font-style: italic;
}

.tree-groups {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}

.tree-group {
  border: 1px solid rgba(0, 0, 0, 0.1);
  border-radius: 0.5rem;
  overflow: hidden;
}

.group-header {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  padding: 0.75rem 1rem;
  background: var(--background-200);
  cursor: pointer;
  user-select: none;
  transition: background 0.2s;

  &:hover {
    background: var(--background-100);
  }
}

.expand-icon {
  display: flex;
  align-items: center;
  justify-content: center;
  transition: transform 0.2s;
  color: var(--text-500);

  &.expanded {
    transform: rotate(90deg);
  }
}

.group-name {
  font-weight: 600;
  color: var(--text-950);
}

.client-count {
  font-size: 0.8rem;
  color: var(--text-500);
}

.group-clients {
  background: var(--background-50);
  border-top: 1px solid var(--color-border);
}

.client-row {
  cursor: pointer;
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 0.5rem 1rem 0.5rem 2rem;
  border-bottom: 1px solid rgba(0, 0, 0, 0.05);

  &:hover {
    background: var(--background-100);
  }

  &:last-child {
    border-bottom: none;
  }
}

.client-name {
  font-size: 0.9rem;
  color: var(--text-900);
}

.status-badge {
  display: inline-block;
  width: fit-content;
  padding: 0.1rem 0.5rem;
  border-radius: 9999px;
  font-weight: 600;
  font-size: 0.7rem;
  text-transform: uppercase;

  &.pending {
    background: var(--status-pending-bg);
    color: var(--status-pending-text);
  }

  &.running,
  &.active {
    background: var(--status-running-bg);
    color: var(--status-running-text);
  }

  &.completed {
    background: var(--status-completed-bg);
    color: var(--status-completed-text);
  }

  &.failed {
    background: var(--status-failed-bg);
    color: var(--status-failed-text);
  }
}

.spinner-small {
  width: 1rem;
  height: 1rem;
  border: 2px solid rgba(0, 0, 0, 0.1);
  border-top-color: var(--accent-500);
  border-radius: 50%;
  animation: spin 0.8s linear infinite;
}

@keyframes spin {
  to {
    transform: rotate(360deg);
  }
}
</style>
