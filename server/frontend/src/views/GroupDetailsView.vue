<script setup lang="ts">
import { ref, computed } from 'vue';
import { useRoute, useRouter } from 'vue-router';
import { useQuery, useMutation, useQueryClient } from '@tanstack/vue-query';
import {
  getGroupById,
  getGroupClients,
  getGroupJobs,
  getClients,
  addClientsToGroup,
  removeClientsFromGroup,
  type Group,
  type ClientSrv,
  type JobSrv
} from '@/client/index';
import { formatDate } from '@/utils/date';
import ConfirmationModal from '@/components/ConfirmationModal.vue';

interface GroupClient {
  id: string;
  client_name: string;
}

const route = useRoute();
const router = useRouter();
const queryClient = useQueryClient();
const groupId = route.params.id as string;

const showDeleteModal = ref(false);

// Fetch Group Details
const { data: group, isLoading: loadingGroup, isError: groupError } = useQuery({
  queryKey: ['group', groupId],
  queryFn: async () => {
    const { data, error } = await getGroupById({ path: { group_id: groupId } });
    if (error) throw error;
    return data as Group;
  }
});

// Fetch Clients in Group
const { data: groupClients, isLoading: loadingClients, refetch: refetchClients } = useQuery({
  queryKey: ['group-clients', groupId],
  queryFn: async () => {
    const { data, error } = await getGroupClients({ path: { group_id: groupId } });
    if (error) return [];
    return (data as ClientSrv[]) || [];
  }
});

// Fetch Jobs in Group
const { data: jobs, isLoading: loadingJobs } = useQuery({
  queryKey: ['group-jobs', groupId],
  queryFn: async () => {
    const { data, error } = await getGroupJobs({ path: { group_id: groupId } });
    if (error) return [];
    return (data as JobSrv[]) || [];
  }
});

// Fetch all available clients for selection
const { data: allClients } = useQuery({
  queryKey: ['clients'],
  queryFn: async () => {
    const { data, error } = await getClients();
    if (error) throw error;
    return (data as ClientSrv[]) || [];
  }
});

const isEditing = ref(false);
const editForm = ref({
  id: '',
  group_name: ''
});
const selectedClientIds = ref<string[]>([]);

const startEditing = () => {
  if (group.value) {
    editForm.value = {
      id: group.value.id,
      group_name: group.value.group_name
    };
    selectedClientIds.value = groupClients.value?.map(c => c.id) || [];
    isEditing.value = true;
  }
};

const addClientsMutation = useMutation({
  mutationFn: async (clientIds: string[]) => {
    const { error } = await addClientsToGroup({
      path: { group_id: groupId },
      body: { client_ids: clientIds }
    });
    if (error) throw error;
  },
  onSuccess: () => {
    refetchClients();
    queryClient.invalidateQueries({ queryKey: ['group', groupId] });
  }
});

const removeClientsMutation = useMutation({
  mutationFn: async (clientIds: string[]) => {
    const { error } = await removeClientsFromGroup({
      path: { group_id: groupId },
      body: { client_ids: clientIds }
    });
    if (error) throw error;
  },
  onSuccess: () => {
    refetchClients();
    queryClient.invalidateQueries({ queryKey: ['group', groupId] });
  }
});

const handleUpdate = () => {
  // For now, group editing just closes the edit mode
  // A proper update would need a backend endpoint
  isEditing.value = false;
  
  // Handle client changes
  const currentClientIds = groupClients.value?.map(c => c.id) || [];
  const newClientIds = selectedClientIds.value;
  
  const clientsToAdd = newClientIds.filter(id => !currentClientIds.includes(id));
  const clientsToRemove = currentClientIds.filter(id => !newClientIds.includes(id));
  
  if (clientsToAdd.length > 0) {
    addClientsMutation.mutate(clientsToAdd);
  }
  if (clientsToRemove.length > 0) {
    removeClientsMutation.mutate(clientsToRemove);
  }
};

const handleDelete = () => {
  // Would need a delete group endpoint
  // For now, just redirect
  router.push('/groups');
};
</script>

<template>
  <div class="page">
    <header class="page-header">
      <router-link to="/groups" class="back-link">← Back to Groups</router-link>
      <div v-if="group" class="header-main">
        <div class="title-group">
          <h1 v-if="!isEditing" class="page-title">{{ group.group_name }}</h1>
          <input 
            v-else
            v-model="editForm.group_name"
            type="text"
            class="edit-title-input"
            placeholder="Group Name"
          />
          <p class="group-id">{{ group.id }}</p>
        </div>
        <div class="header-actions">
          <button v-if="!isEditing" @click="startEditing" class="btn-secondary">Edit Group</button>
          <button v-if="!isEditing" @click="showDeleteModal = true" class="btn-danger">Delete Group</button>
        </div>
      </div>
    </header>

    <div v-if="loadingGroup" class="state-card">
      <div class="spinner" />
      <p>Loading group details...</p>
    </div>

    <div v-else-if="groupError || !group" class="state-card state-error">
      <p class="state-label">Group not found</p>
      <p class="state-text">The requested group could not be retrieved.</p>
    </div>

    <div v-else class="details-container">
      <!-- General Info Section -->
      <section class="info-section card">
        <div class="section-header">
          <h2>General Information</h2>
        </div>

        <div v-if="isEditing" class="edit-form-actions">
          <button type="button" @click="isEditing = false" class="btn-ghost">Cancel</button>
          <button 
            type="button" 
            @click="handleUpdate" 
            class="btn-primary"
            :disabled="addClientsMutation.isPending.value || removeClientsMutation.isPending.value"
          >
            {{ addClientsMutation.isPending.value || removeClientsMutation.isPending.value ? 'Saving...' : 'Save Changes' }}
          </button>
        </div>

        <div class="info-grid">
          <div class="info-item">
            <span class="label">Group Name</span>
            <span v-if="!isEditing" class="value">{{ group.group_name }}</span>
          </div>
          <div class="info-item">
            <span class="label">ID</span>
            <span class="value monospace">{{ group.id }}</span>
          </div>
          <div class="info-item">
            <span class="label">Created At</span>
            <span class="value">{{ formatDate(group.created_at) }}</span>
          </div>
          <div class="info-item">
            <span class="label">Updated At</span>
            <span class="value">{{ formatDate(group.updated_at) }}</span>
          </div>
        </div>
      </section>

      <!-- Client Selection Section (only shown when editing) -->
      <section v-if="isEditing" class="info-section card">
        <div class="section-header">
          <h2>Assigned Clients</h2>
        </div>

        <div v-if="!allClients || allClients.length === 0" class="empty-text">
          No clients available. Clients will appear here once they connect.
        </div>

        <div v-else class="client-selector">
          <div v-for="client in allClients" :key="client.id" class="client-checkbox">
            <input 
              type="checkbox" 
              :id="client.id" 
              :value="client.id" 
              v-model="selectedClientIds" 
            />
            <label :for="client.id">
              <span class="client-name">{{ client.client_name }}</span>
              <span class="client-id">{{ client.id }}</span>
            </label>
          </div>
        </div>
      </section>

      <!-- Clients Section (read-only) -->
      <section v-if="!isEditing" class="clients-section card">
        <div class="section-header">
          <h2>Clients</h2>
          <span v-if="groupClients && groupClients.length > 0" class="count-badge">{{ groupClients.length }}</span>
        </div>

        <div v-if="loadingClients" class="loading-state">
          <div class="spinner-small" />
          <span>Loading clients...</span>
        </div>

        <div v-else-if="!groupClients || groupClients.length === 0" class="empty-text">
          No clients assigned to this group.
        </div>

        <div v-else class="entity-list">
          <router-link
            v-for="client in groupClients"
            :key="client.id"
            :to="'/clients/' + client.id"
            class="entity-chip"
          >
            <span class="entity-name">{{ client.client_name }}</span>
            <span class="entity-id">{{ client.id }}</span>
          </router-link>
        </div>
      </section>

      <!-- Jobs Section -->
      <section class="jobs-section card">
        <div class="section-header">
          <h2>Jobs</h2>
          <span v-if="jobs && jobs.length > 0" class="count-badge">{{ jobs.length }}</span>
        </div>

        <div v-if="loadingJobs" class="loading-state">
          <div class="spinner-small" />
          <span>Loading jobs...</span>
        </div>

        <div v-else-if="!jobs || jobs.length === 0" class="empty-text">
          No jobs assigned to this group.
        </div>

        <div v-else class="entity-list">
          <router-link
            v-for="job in jobs"
            :key="job.id"
            :to="'/jobs/' + job.id"
            class="entity-chip"
          >
            <span class="entity-name">{{ job.job_name }}</span>
            <span class="entity-id">{{ job.id }}</span>
          </router-link>
        </div>
      </section>
    </div>

    <ConfirmationModal 
      v-if="showDeleteModal" 
      title="Delete Group"
      message="Are you sure you want to delete this group? This action cannot be undone." 
      confirm-text="Delete"
      variant="danger" 
      @confirm="handleDelete" 
      @cancel="showDeleteModal = false" 
    />
  </div>
</template>

<style lang="scss" scoped>
.page {
  display: flex;
  flex-direction: column;
  width: 100%;
  max-width: 1000px;
  margin: 0 auto;
  padding: 2rem 1.5rem;
  gap: 2rem;
}

.back-link {
  display: inline-block;
  margin-bottom: 1rem;
  font-size: 0.9rem;
  color: var(--accent-500);
  text-decoration: none;

  &:hover {
    text-decoration: underline;
  }
}

.header-main {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  gap: 1rem;
}

.header-actions {
  display: flex;
  gap: 0.75rem;
}

.title-group {
  .page-title {
    margin: 0;
    font-size: 2rem;
    font-weight: 800;
  }

  .edit-title-input {
    margin: 0;
    font-size: 2rem;
    font-weight: 800;
    padding: 0.25rem 0.5rem;
    border: 1px solid var(--accent-500);
    border-radius: 0.25rem;
    background: var(--background-800);
    color: var(--text-950);
  }

  .group-id {
    margin: 0.25rem 0 0;
    font-size: 0.9rem;
    font-family: monospace;
    opacity: 0.5;
  }
}

.details-container {
  display: flex;
  flex-direction: column;
  gap: 1.5rem;
}

.card {
  background: var(--background-300);
  color: var(--text-950);
  border-radius: 1rem;
  padding: 1.5rem;
  box-shadow: 0 4px 6px -1px rgba(0, 0, 0, 0.1);
}

.section-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 1.5rem;
  border-bottom: 1px solid rgba(0, 0, 0, 0.1);
  padding-bottom: 0.75rem;

  h2 {
    margin: 0;
    font-size: 1.25rem;
    font-weight: 700;
  }
}

.count-badge {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  min-width: 1.5rem;
  height: 1.5rem;
  padding: 0 0.5rem;
  background: var(--background-200);
  border-radius: 9999px;
  font-size: 0.8rem;
  font-weight: 600;
  color: var(--text-700);
}

.info-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
  gap: 1.5rem;
}

.info-item {
  display: flex;
  flex-direction: column;
  gap: 0.25rem;

  .label {
    font-size: 0.7rem;
    font-weight: 700;
    text-transform: uppercase;
    opacity: 0.6;
  }

  .value {
    font-size: 1rem;

    &.monospace {
      font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, "Liberation Mono", "Courier New", monospace;
      font-size: 0.85rem;
      background: rgba(0, 0, 0, 0.05);
      padding: 0.2rem 0.4rem;
      border-radius: 0.25rem;
      word-break: break-all;
    }
  }
}

.client-selector {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
  max-height: 250px;
  overflow-y: auto;
  padding: 1rem;
  background: var(--background-50);
  border-radius: 0.5rem;
  border: 1px solid var(--color-border);
}

.client-checkbox {
  display: flex;
  align-items: center;
  gap: 0.75rem;
  padding: 0.5rem;
  border-radius: 0.25rem;

  &:hover {
    background: var(--background-50);
  }

  input {
    width: 1.1rem;
    height: 1.1rem;
    cursor: pointer;
  }

  label {
    display: flex;
    flex-direction: column;
    cursor: pointer;
    text-transform: none;
    letter-spacing: normal;
    opacity: 1;

    .client-name {
      font-weight: 600;
      font-size: 0.95rem;
    }

    .client-id {
      font-size: 0.75rem;
      opacity: 0.5;
      font-family: monospace;
    }
  }
}

.entity-list {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}

.entity-chip {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 0.75rem 1rem;
  background: var(--background-200);
  border: 1px solid rgba(0, 0, 0, 0.1);
  border-radius: 0.5rem;
  text-decoration: none;
  color: inherit;
  transition: background 0.2s, border-color 0.2s;

  &:hover {
    background: var(--background-100);
    border-color: var(--accent-400);
  }

  .entity-name {
    font-weight: 600;
    color: var(--text-900);
  }

  .entity-id {
    font-size: 0.8rem;
    font-family: monospace;
    opacity: 0.5;
  }
}

.loading-state {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  padding: 1rem;
  color: var(--text-500);
  font-size: 0.9rem;
}

.empty-text {
  color: var(--text-500);
  font-size: 0.9rem;
  font-style: italic;
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

.spinner {
  width: 2rem;
  height: 2rem;
  border: 3px solid rgba(0, 0, 0, 0.1);
  border-top-color: var(--accent-500);
  border-radius: 50%;
  animation: spin 0.8s linear infinite;
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

.btn-primary {
  background: var(--accent-500);
  color: white;
  padding: 0.6rem 1.25rem;
  border-radius: 0.5rem;
  font-weight: 700;
  border: none;
  cursor: pointer;
}

.btn-secondary {
  background: transparent;
  border: 1px solid var(--accent-500);
  color: var(--accent-500);
  padding: 0.6rem 1.25rem;
  border-radius: 0.5rem;
  font-weight: 600;
  cursor: pointer;
}

.btn-ghost {
  background: transparent;
  border: none;
  padding: 0.6rem 1.25rem;
  cursor: pointer;
  color: var(--text-800);
}

.btn-danger {
  background: var(--danger-bg);
  color: var(--danger-text);
  border: 1px solid var(--danger-text);
  padding: 0.6rem 1.25rem;
  border-radius: 0.5rem;
  font-weight: 600;
  cursor: pointer;

  &:hover {
    opacity: 0.9;
  }
}

.edit-form-actions {
  display: flex;
  justify-content: flex-end;
  gap: 0.75rem;
  margin-bottom: 1rem;
}
</style>