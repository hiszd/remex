<script setup lang="ts">
import { ref } from 'vue';
import { useRouter } from 'vue-router';
import { useQuery, useMutation, useQueryClient } from '@tanstack/vue-query';
import { getClients, createGroup } from '@/client/index';
import type { Client, CreateGroupForm } from '@/client/types.gen';

const router = useRouter();
const queryClient = useQueryClient();

const { data: clients, isLoading: loadingClients } = useQuery({
  queryKey: ['clients'],
  queryFn: async () => {
    const { data, error } = await getClients();
    if (error) throw error;
    return data as Client[];
  }
});

const form = ref({
  group_name: '',
  client_ids: [] as string[]
});

const isSubmitting = ref(false);
const submitError = ref<string | null>(null);

const mutation = useMutation({
  mutationFn: async (newGroup: CreateGroupForm) => {
    const { data, error } = await createGroup({
      body: newGroup
    });
    if (error) throw error;
    return data;
  },
  onSuccess: () => {
    queryClient.invalidateQueries({ queryKey: ['groups'] });
    queryClient.invalidateQueries({ queryKey: ['clients'] });
    router.push('/groups');
  },
  onError: (err: Error) => {
    submitError.value = err.message || 'Failed to create group';
    isSubmitting.value = false;
  }
});

const handleSubmit = async () => {
  isSubmitting.value = true;
  submitError.value = null;
  const payload: CreateGroupForm = {
    group_name: form.value.group_name,
    client_ids: form.value.client_ids.length > 0 ? form.value.client_ids : null
  };
  mutation.mutate(payload);
};
</script>

<template>
  <div class="page">
    <header class="page-header">
      <router-link to="/groups" class="back-link">← Back to Groups</router-link>
      <h1 class="page-title">Create New Group</h1>
      <p class="page-subtitle">Create a new group to organize your clients.</p>
    </header>

    <form @submit.prevent="handleSubmit" class="form-card">
      <div v-if="submitError" class="error-banner">
        {{ submitError }}
      </div>

      <div class="form-section">
        <label for="group_name">Group Name</label>
        <input 
          id="group_name"
          v-model="form.group_name"
          type="text"
          placeholder="e.g. Production Servers"
          required
        />
      </div>

      <div class="form-section">
        <div class="label-row">
          <label>Assign Clients</label>
        </div>

        <div v-if="loadingClients" class="loading-inline">Loading clients...</div>
        <div v-else class="client-selector">
          <div v-for="client in clients" :key="client.id" class="client-checkbox">
            <input type="checkbox" :id="client.id" :value="client.id" v-model="form.client_ids" />
            <label :for="client.id">
              <span class="client-name">{{ client.client_name }}</span>
              <span class="client-id">{{ client.id }}</span>
            </label>
          </div>
          <div v-if="!clients || clients.length === 0" class="no-clients">
            No clients available. Clients will appear here once they connect.
          </div>
        </div>
      </div>

      <div class="form-actions">
        <button type="button" @click="router.push('/groups')" class="btn-secondary" :disabled="isSubmitting">
          Cancel
        </button>
        <button type="submit" class="btn-primary" :disabled="isSubmitting">
          {{ isSubmitting ? 'Creating...' : 'Create Group' }}
        </button>
      </div>
    </form>
  </div>
</template>

<style lang="scss" scoped>
.page {
  display: flex;
  flex-direction: column;
  width: 100%;
  max-width: 800px;
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

.page-header {
  .page-title {
    margin: 0;
    font-size: 1.75rem;
    font-weight: 800;
  }

  .page-subtitle {
    margin: 0.25rem 0 0;
    font-size: 0.95rem;
    color: var(--text-500);
  }
}

.form-card {
  display: flex;
  flex-direction: column;
  gap: 1.5rem;
  padding: 2rem;
  border-radius: 1rem;
  background-color: var(--background-300);
  color: var(--text-950);
  box-shadow: 0 4px 6px -1px rgba(0, 0, 0, 0.1);
}

.error-banner {
  padding: 1rem;
  background-color: var(--accent-50);
  border: 1px solid var(--accent-400);
  color: var(--accent-900);
  border-radius: 0.5rem;
  font-size: 0.9rem;
}

.form-section {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;

  label {
    font-size: 0.85rem;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.025em;
    opacity: 0.8;
  }

  input[type="text"] {
    padding: 0.75rem 1rem;
    border-radius: 0.5rem;
    border: 1px solid var(--color-border);
    background: var(--background-50);
    font-size: 1rem;
    color: var(--text-950);

    &:focus {
      outline: none;
      border-color: var(--accent-500);
      box-shadow: 0 0 0 2px var(--accent-100);
    }
  }
}

.label-row {
  display: flex;
  align-items: center;
  justify-content: space-between;

  label {
    font-size: 0.85rem;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.025em;
    opacity: 0.8;
  }
}

.loading-inline {
  padding: 1rem;
  font-size: 0.9rem;
  color: var(--text-500);
  text-align: center;
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

.no-clients {
  font-size: 0.9rem;
  color: var(--text-500);
  text-align: center;
  padding: 1rem;
}

.form-actions {
  display: flex;
  justify-content: flex-end;
  gap: 1rem;
  margin-top: 1rem;
  padding-top: 1.5rem;
  border-top: 1px solid rgba(0, 0, 0, 0.1);
}

.btn-primary {
  padding: 0.75rem 2rem;
  border-radius: 0.5rem;
  background-color: var(--accent-500);
  color: white;
  font-weight: 700;
  border: none;
  cursor: pointer;
  transition: background 0.2s;

  &:hover:not(:disabled) {
    background-color: var(--accent-600);
  }

  &:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
}

.btn-secondary {
  padding: 0.75rem 2rem;
  border-radius: 0.5rem;
  background-color: transparent;
  color: var(--text-950);
  font-weight: 600;
  border: 1px solid rgba(0, 0, 0, 0.1);
  cursor: pointer;

  &:hover:not(:disabled) {
    background-color: rgba(0, 0, 0, 0.05);
  }
}
</style>
