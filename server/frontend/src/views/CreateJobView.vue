<script setup lang="ts">
import { ref, computed } from 'vue';
import { useRouter } from 'vue-router';
import { useQuery, useMutation, useQueryClient } from '@tanstack/vue-query';
import { getGroups, createJob } from '@/client/index';
import type { CreateJobForm, Group } from '@/client/types.gen';

const router = useRouter();
const queryClient = useQueryClient();

const { data: groups, isLoading: loadingGroups } = useQuery({
  queryKey: ['groups'],
  queryFn: async () => {
    const { data, error } = await getGroups();
    if (error) throw error;
    return data as Group[];
  }
});

const form = ref({
  job_name: '',
  job_type: 'instant',
  job_status: 'pending',
  job_shell: '',
  job_command: '',
  group_ids: [] as string[],
  scheduled_at: null as string | null,
  frequency_number: 1,
  frequency_unit: 'hours'
});

const frequencyIso8601 = computed(() => {
  if (form.value.job_type !== 'recurring') return null;
  const num = form.value.frequency_number;
  const unit = form.value.frequency_unit;
  if (unit === 'minutes') return `PT${num}M`;
  if (unit === 'hours') return `PT${num}H`;
  if (unit === 'days') return `P${num}D`;
  if (unit === 'weeks') return `P${num}W`;
  return null;
});

const isSubmitting = ref(false);
const submitError = ref<string | null>(null);

const mutation = useMutation({
  mutationFn: async (newJob: CreateJobForm) => {
    const { data, error } = await createJob({
      body: newJob
    });
    if (error) throw error;
    return data;
  },
  onSuccess: () => {
    queryClient.invalidateQueries({ queryKey: ['jobs'] });
    router.push('/jobs');
  },
  onError: (err: Error) => {
    submitError.value = err.message || 'Failed to create job';
    isSubmitting.value = false;
  }
});

const handleSubmit = async () => {
  isSubmitting.value = true;
  submitError.value = null;
  const payload: CreateJobForm = {
    job_name: form.value.job_name,
    job_type: form.value.job_type,
    job_status: form.value.job_status as 'pending' | 'running' | 'completed' | 'cancelled' | 'timed_out' | 'disabled',
    job_shell: form.value.job_shell,
    job_command: form.value.job_command,
    group_ids: form.value.group_ids,
    scheduled_at: (form.value.job_type === 'scheduled' || form.value.job_type === 'recurring') ? form.value.scheduled_at : null,
    frequency: frequencyIso8601.value
  };
  mutation.mutate(payload);
};
</script>

<template>
  <div class="page">
    <header class="page-header">
      <router-link to="/jobs" class="back-link">← Back to Jobs</router-link>
      <h1 class="page-title">Create New Job</h1>
      <p class="page-subtitle">Configure a new task to be executed by your clients.</p>
    </header>

    <form @submit.prevent="handleSubmit" class="form-card">
      <div v-if="submitError" class="error-banner">
        {{ submitError }}
      </div>

      <div class="form-section">
        <label for="job_name">Job Name</label>
        <input id="job_name" v-model="form.job_name" type="text" placeholder="e.g. System Update" required />
      </div>

      <div class="form-row">
        <div class="form-section">
          <label for="job_type">Type</label>
          <select id="job_type" v-model="form.job_type">
            <option value="instant">Instant</option>
            <option value="scheduled">Scheduled</option>
            <option value="recurring">Recurring</option>
          </select>
        </div>

        <div class="form-section">
          <label for="job_status">Initial Status</label>
          <select id="job_status" v-model="form.job_status">
            <option value="pending">Pending</option>
            <option value="running">Running</option>
            <option value="completed">Completed</option>
            <option value="failed">Failed</option>
            <option value="cancelled">Cancelled</option>
            <option value="timed_out">Timed Out</option>
            <option value="disabled">Disabled</option>
          </select>
        </div>

        <template v-if="form.job_type === 'scheduled' || form.job_type === 'recurring'">
          <div class="form-section">
            <label for="scheduled_at">Scheduled Time</label>
            <input type="datetime-local" id="scheduled_at" v-model="form.scheduled_at" required />
            <span class="field-desc">Format: YYYY-MM-DDTHH:MM:SS</span>
          </div>
        </template>

        <template v-if="form.job_type === 'recurring'">
          <div class="form-row">
            <div class="form-section">
              <label for="frequency_number">Repeat Every</label>
              <input type="number" id="frequency_number" v-model="form.frequency_number" min="1" required />
            </div>
            <div class="form-section">
              <label for="frequency_unit">&nbsp;</label>
              <select id="frequency_unit" v-model="form.frequency_unit">
                <option value="minutes">Minutes</option>
                <option value="hours">Hours</option>
                <option value="days">Days</option>
                <option value="weeks">Weeks</option>
              </select>
            </div>
          </div>
        </template>
      </div>

      <div class="form-section">
        <label for="job_shell">Shell</label>
        <input type="text" id="job_shell" v-model="form.job_shell" placeholder="e.g. /bin/bash" required />
      </div>

      <div class="form-section">
        <label for="job_command">Command</label>
        <textarea id="job_command" v-model="form.job_command" placeholder="e.g. apt-get update && apt-get upgrade -y"
          rows="4" required></textarea>
      </div>

      <!-- Assign to Groups -->
      <div class="form-section">
        <div class="label-row">
          <label>Assign to Groups</label>
          <a href="/groups/new" target="_blank" class="create-link">+ Create New Group</a>
        </div>

        <div v-if="loadingGroups" class="loading-inline">Loading groups...</div>
        <div v-else class="group-selector">
          <div v-for="group in groups" :key="group.id" class="group-checkbox">
            <input type="checkbox" :id="group.id" :value="group.id" v-model="form.group_ids" />
            <label :for="group.id">
              <span class="group-name">{{ group.group_name }}</span>
              <span class="group-id">{{ group.id }}</span>
            </label>
          </div>
          <div v-if="!groups || groups.length === 0" class="no-groups">
            No groups available. <a href="/groups/new" target="_blank">Create a group</a> to assign jobs to.
          </div>
        </div>
      </div>

      <div class="form-actions">
        <button type="button" @click="router.push('/jobs')" class="btn-secondary" :disabled="isSubmitting">
          Cancel
        </button>
        <button type="submit" class="btn-primary" :disabled="isSubmitting">
          {{ isSubmitting ? 'Creating...' : 'Create Job' }}
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

  .field-desc {
    margin: -0.25rem 0 0.5rem;
    font-size: 0.8rem;
    opacity: 0.6;
    font-style: italic;
  }

  input[type="text"],
  input[type="datetime-local"],
  input[type="number"],
  select,
  textarea {
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

  input[type="number"] {
    -webkit-inner-spin-button,
    -webkit-outer-spin-button {
      -webkit-appearance: none;
      margin: 0;
    }
    -moz-appearance: textfield;
  }
}

.form-row {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 1.5rem;
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

.create-link {
  font-size: 0.85rem;
  font-weight: 600;
  text-transform: none;
  letter-spacing: normal;
  opacity: 1;
  color: var(--accent-500);
  text-decoration: none;

  &:hover {
    text-decoration: underline;
  }
}

.group-selector {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
  max-height: 200px;
  overflow-y: auto;
  padding: 1rem;
  background: var(--background-50);
  border-radius: 0.5rem;
  border: 1px solid var(--color-border);
}

.group-checkbox {
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

    .group-name {
      font-weight: 600;
      font-size: 0.95rem;
    }

    .group-id {
      font-size: 0.75rem;
      opacity: 0.5;
      font-family: monospace;
    }
  }
}

.no-groups {
  font-size: 0.9rem;
  color: var(--text-500);
  text-align: center;
  padding: 1rem;

  a {
    color: var(--accent-500);
    text-decoration: none;

    &:hover {
      text-decoration: underline;
    }
  }
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
