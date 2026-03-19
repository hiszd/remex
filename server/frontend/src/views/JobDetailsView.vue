<script setup lang="ts">
import { ref, computed } from 'vue';
import { useRoute } from 'vue-router';
import { useQuery, useMutation, useQueryClient } from '@tanstack/vue-query';
import {
  getJobById,
  getJobGroups,
  getGroupClients,
  getGroups,
  updateJob,
  updateJobGroups
} from '@/client/index';
import type { JobWithClients, UpdateJobSrv, JobWithGroups, ClientSrv, Group, UpdateJobGroupsForm } from '@/client/types.gen';
import { formatDate } from '@/utils/date';
import ClientTree from '@/components/ClientTree.vue';

interface GroupWithClients {
  id: string;
  group_name: string;
  clients: ClientSrv[];
}

const route = useRoute();
const queryClient = useQueryClient();
const jobId = route.params.id as string;

// Fetch Job Details
const { data: job, isLoading: loadingJob, isError: jobError } = useQuery({
  queryKey: ['job', jobId],
  queryFn: async () => {
    const { data, error } = await getJobById({ path: { id: jobId } });
    if (error) throw error;
    return data as JobWithClients;
  }
});

// Fetch Groups for Job
const { data: groups, isLoading: loadingGroups } = useQuery({
  queryKey: ['job-groups', jobId],
  queryFn: async () => {
    const { data, error } = await getJobGroups({ path: { id: jobId } });
    if (error) return [];
    return (data as JobWithGroups[]) || [];
  }
});

// Fetch all available groups for selection
const { data: allGroups } = useQuery({
  queryKey: ['groups'],
  queryFn: async () => {
    const { data, error } = await getGroups();
    if (error) throw error;
    return (data as Group[]) || [];
  }
});

// Fetch clients for each group
const { data: groupsWithClients } = useQuery({
  queryKey: ['job-groups-clients', jobId],
  queryFn: async () => {
    if (!groups.value) return [];
    const result: GroupWithClients[] = [];
    for (const group of groups.value) {
      const { data: clients } = await getGroupClients({ path: { group_id: group.id } });
      result.push({
        id: group.id,
        group_name: group.group_name,
        clients: (clients as ClientSrv[]) || [],
      });
    }
    return result;
  },
  enabled: computed(() => (groups.value?.length ?? 0) > 0)
});

const isEditing = ref(false);
const editForm = ref<UpdateJobSrv>({
  id: '',
  job_name: '',
  job_type: '',
  job_status: '',
  job_shell: ''
});
const selectedGroupIds = ref<string[]>([]);

const startEditing = () => {
  if (job.value) {
    editForm.value = {
      id: job.value.id,
      job_name: job.value.job_name,
      job_type: job.value.job_type,
      job_status: job.value.job_status,
      job_shell: job.value.job_shell
    };
    selectedGroupIds.value = groups.value?.map(g => g.id) || [];
    isEditing.value = true;
  }
};

const updateMutation = useMutation({
  mutationFn: async (updatedJob: UpdateJobSrv) => {
    const { data, error } = await updateJob({ body: updatedJob });
    if (error) throw error;
    return data;
  },
  onSuccess: () => {
    queryClient.invalidateQueries({ queryKey: ['job', jobId] });
  }
});

const updateGroupsMutation = useMutation({
  mutationFn: async (groupIds: UpdateJobGroupsForm) => {
    const { error } = await updateJobGroups({
      path: { job_id: jobId },
      body: groupIds
    });
    if (error) throw error;
  },
  onSuccess: () => {
    queryClient.invalidateQueries({ queryKey: ['job-groups', jobId] });
    queryClient.invalidateQueries({ queryKey: ['job-groups-clients', jobId] });
  }
});

const handleUpdate = () => {
  updateMutation.mutate(editForm.value);
  updateGroupsMutation.mutate({ group_ids: selectedGroupIds.value });
  isEditing.value = false;
};
</script>

<template>
  <div class="page">
    <header class="page-header">
      <router-link to="/jobs" class="back-link">← Back to Jobs</router-link>
      <div class="header-main">
        <div v-if="job" class="title-group">
          <h1 class="page-title">{{ job.job_name }}</h1>
          <p class="job-id">{{ job.id }}</p>
        </div>
        <button v-if="job && !isEditing" @click="startEditing" class="btn-secondary">Edit Job</button>
      </div>
    </header>

    <div v-if="loadingJob" class="state-card">
      <div class="spinner" />
      <p>Loading job details...</p>
    </div>

    <div v-else-if="jobError || !job" class="state-card state-error">
      <p class="state-label">Job not found</p>
      <p class="state-text">The requested job could not be retrieved.</p>
    </div>

    <div v-else class="details-container">
      <!-- Job Info Section -->
      <section class="info-section card">
        <div class="section-header">
          <h2>General Information</h2>
        </div>

        <form v-if="isEditing" @submit.prevent="handleUpdate" class="edit-form">
          <div class="form-grid">
            <div class="form-group">
              <label>Name</label>
              <input v-model="editForm.job_name" type="text" required />
            </div>
            <div class="form-group">
              <label>Type</label>
              <select v-model="editForm.job_type">
                <option value="Shell">Shell</option>
                <option value="Update">Update</option>
                <option value="Internal">Internal</option>
              </select>
            </div>
            <div class="form-group">
              <label>Status</label>
              <select v-model="editForm.job_status">
                <option value="Pending">Pending</option>
                <option value="Running">Running</option>
                <option value="Paused">Paused</option>
                <option value="Completed">Completed</option>
                <option value="Failed">Failed</option>
              </select>
            </div>
          </div>
          <div class="form-group">
            <label>Shell Command</label>
            <textarea v-model="editForm.job_shell" rows="4" required></textarea>
          </div>
          <div class="form-actions">
            <button type="button" @click="isEditing = false" class="btn-ghost">Cancel</button>
            <button type="submit" class="btn-primary" :disabled="updateMutation.isPending.value">
              {{ updateMutation.isPending.value ? 'Saving...' : 'Save Changes' }}
            </button>
          </div>
        </form>

        <div v-else class="read-only-info">
          <div class="info-grid">
            <div class="info-item">
              <span class="label">Status</span>
              <span class="value status-badge" :class="job.job_status.toLowerCase()">{{ job.job_status }}</span>
            </div>
            <div class="info-item">
              <span class="label">Type</span>
              <span class="value">{{ job.job_type }}</span>
            </div>
            <div class="info-item">
              <span class="label">Created At</span>
              <span class="value">{{ formatDate(job.created_at) }}</span>
            </div>
            <div class="info-item">
              <span class="label">Updated At</span>
              <span class="value">{{ formatDate(job.updated_at) }}</span>
            </div>
          </div>
          <div class="info-item full-width">
            <span class="label">Shell Command</span>
            <pre class="command-box">{{ job.job_shell }}</pre>
          </div>
        </div>
      </section>

      <!-- Group Assignment Section (only shown when editing) -->
      <section v-if="isEditing" class="info-section card">
        <div class="section-header">
          <h2>Assigned Groups</h2>
          <a href="/groups/new" target="_blank" class="create-link">+ Create New Group</a>
        </div>
        <div class="group-selector">
          <div v-if="!allGroups || allGroups.length === 0" class="no-groups">
            No groups available. <a href="/groups/new" target="_blank">Create a group</a> first.
          </div>
          <div v-else v-for="group in allGroups" :key="group.id" class="group-checkbox">
            <input type="checkbox" :id="group.id" :value="group.id" v-model="selectedGroupIds" />
            <label :for="group.id">
              <span class="group-name">{{ group.group_name }}</span>
              <span class="group-id">{{ group.id }}</span>
            </label>
          </div>
        </div>
      </section>

      <!-- Client Tree Section -->
      <section v-if="!isEditing" class="client-tree-section card">
        <div class="section-header">
          <h2>Client Groups</h2>
        </div>
        <ClientTree :groups="groupsWithClients || []" :job-id="jobId" />
      </section>
    </div>
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

.title-group {
  .page-title {
    margin: 0;
    font-size: 2rem;
    font-weight: 800;
  }

  .job-id {
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

/* Info Grid */
.info-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
  gap: 1.5rem;
  margin-bottom: 1.5rem;
}

.info-item {
  display: flex;
  flex-direction: column;
  gap: 0.25rem;

  &.full-width {
    grid-column: 1 / -1;
  }

  .label {
    font-size: 0.7rem;
    font-weight: 700;
    text-transform: uppercase;
    opacity: 0.6;
  }

  .value {
    font-size: 1rem;
  }
}

.status-badge {
  display: inline-block;
  width: fit-content;
  padding: 0.1rem 0.6rem;
  border-radius: 9999px;
  font-weight: 600;
  font-size: 0.85rem;
  background: var(--background-200);

  &.active,
  &.running {
    background: var(--status-running-bg);
    color: var(--status-running-text);
  }

  &.pending {
    background: var(--status-pending-bg);
    color: var(--status-pending-text);
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

.command-box {
  background: var(--background-100);
  padding: 1rem;
  border-radius: 0.5rem;
  font-family: ui-monospace, monospace;
  font-size: 0.9rem;
  overflow-x: auto;
  margin: 0;
}

/* Edit Form */
.form-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
  gap: 1rem;
  margin-bottom: 1rem;
}

.form-group {
  display: flex;
  flex-direction: column;
  gap: 0.4rem;
  margin-bottom: 1rem;

  label {
    font-size: 0.8rem;
    font-weight: 700;
    opacity: 0.8;
  }

  input,
  select,
  textarea {
    padding: 0.6rem;
    border-radius: 0.4rem;
    border: 1px solid rgba(0, 0, 0, 0.1);
    font-size: 0.95rem;
  }
}

.create-link {
  font-size: 0.8rem;
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
  max-height: 180px;
  overflow-y: auto;
  padding: 0.75rem;
  background: var(--background-100);
  border-radius: 0.4rem;
  border: 1px solid rgba(0, 0, 0, 0.1);
}

.group-checkbox {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  padding: 0.35rem;
  border-radius: 0.25rem;

  &:hover {
    background: rgba(0, 0, 0, 0.03);
  }

  input {
    width: 1rem;
    height: 1rem;
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
      font-size: 0.9rem;
    }

    .group-id {
      font-size: 0.7rem;
      opacity: 0.4;
      font-family: monospace;
    }
  }
}

.no-groups {
  font-size: 0.85rem;
  color: var(--text-500);
  text-align: center;
  padding: 0.75rem;

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
  gap: 0.75rem;
}

/* Buttons */
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

.loading-text {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  padding: 1rem;
  color: var(--text-500);
  font-size: 0.9rem;
}

.groups-list {
  display: flex;
  flex-wrap: wrap;
  gap: 0.5rem;
}

.group-chip {
  display: inline-block;
  padding: 0.25rem 0.75rem;
  background: var(--background-200);
  border: 1px solid rgba(0, 0, 0, 0.1);
  border-radius: 9999px;
  font-size: 0.85rem;
  font-weight: 600;
  color: var(--text-900);
}

@keyframes spin {
  to {
    transform: rotate(360deg);
  }
}
</style>
