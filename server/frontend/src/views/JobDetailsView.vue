<script setup lang="ts">
import { ref, computed } from 'vue';
import { useRoute, useRouter } from 'vue-router';
import { useQuery, useMutation, useQueryClient } from '@tanstack/vue-query';
import {
  getJobById,
  getJobGroups,
  getJobClientStatuses,
  getGroupClients,
  getGroups,
  updateJob,
  updateJobGroups,
  deleteJob
} from '@/client/index';
import type { JobWithClients, UpdateJobSrv, JobWithGroups, ClientSrv, Group, UpdateJobGroupsForm, ClientJobStatusResponse } from '@/client/types.gen';
import { formatDate } from '@/utils/date';
import ClientTree from '@/components/ClientTree.vue';
import ConfirmationModal from '@/components/ConfirmationModal.vue';

interface GroupWithClients {
  id: string;
  group_name: string;
  clients: ClientSrv[];
}

const route = useRoute();
const router = useRouter();
const queryClient = useQueryClient();
const jobId = route.params.id as string;

const showDeleteModal = ref(false);

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
const { data: groups } = useQuery({
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

const { data: clientStatuses, isLoading: loadingClientStatuses } = useQuery({
  queryKey: ['job-client-statuses', jobId],
  queryFn: async () => {
    const { data, error } = await getJobClientStatuses({ path: { job_id: jobId } });
    if (error) return [];
    return (data as ClientJobStatusResponse[]) || [];
  }
});

const isEditing = ref(false);
const isManagingGroups = ref(false);
const showScheduleWarning = ref(false);
const managingGroupsGroupIds = ref<string[]>([]);

const activeGroupIds = computed({
  get: () => isEditing.value ? selectedGroupIds.value : managingGroupsGroupIds.value,
  set: (val: string[]) => {
    if (isEditing.value) {
      selectedGroupIds.value = val;
    } else {
      managingGroupsGroupIds.value = val;
    }
  }
});
const editForm = ref<UpdateJobSrv & {
  scheduled_at: string | null;
  frequency_number: number;
  frequency_unit: string
}>({
  id: '',
  job_name: '',
  job_type: '',
  job_status: '',
  job_shell: '',
  job_command: '',
  scheduled_at: null,
  frequency_number: 1,
  frequency_unit: 'hours'
});
const selectedGroupIds = ref<string[]>([]);

const parseIso8601ToNumberUnit = (iso: string | null): { frequency_number: number; frequency_unit: string } | null => {
  if (!iso) return null;
  const numMatch = iso.match(/\d+/);
  const num = numMatch ? parseInt(numMatch[0]) : 1;
  if (iso.includes('H')) return { frequency_number: num, frequency_unit: 'hours' };
  if (iso.includes('D')) return { frequency_number: num, frequency_unit: 'days' };
  if (iso.includes('W')) return { frequency_number: num, frequency_unit: 'weeks' };
  if (iso.includes('M')) return { frequency_number: num, frequency_unit: 'minutes' };
  return null;
};

const frequencyToSeconds = (num: number, unit: string): number => {
  if (unit === 'minutes') return num * 60;
  if (unit === 'hours') return num * 3600;
  if (unit === 'days') return num * 86400;
  if (unit === 'weeks') return num * 604800;
  return num * 3600;
};

interface JobTypeInstant {
  instant: null;
}

interface JobTypeScheduled {
  scheduled: string;
}

interface JobTypeRecurring {
  recurring: [string, number];
}

type JobTypeJson = JobTypeInstant | JobTypeScheduled | JobTypeRecurring;

const parsedJobType = computed(() => {
  if (!job.value) return { type: 'instant', scheduledAt: null, frequency_number: null as number | null, frequency_unit: null as string | null };
  const typeStr = job.value.job_type;
  try {
    const parsed = JSON.parse(typeStr) as JobTypeJson;
    if ('instant' in parsed) {
      return { type: 'instant', scheduledAt: null, frequency_number: null, frequency_unit: null };
    }
    if ('scheduled' in parsed) {
      return { type: 'scheduled', scheduledAt: parsed.scheduled, frequency_number: null, frequency_unit: null };
    }
    if ('recurring' in parsed) {
      const [scheduledAt, durationSecs] = parsed.recurring;
      return {
        type: 'recurring',
        scheduledAt,
        frequency_number: Math.ceil(durationSecs / 60),
        frequency_unit: durationSecs < 3600 ? 'minutes' : 'hours'
      };
    }
  } catch {
  }
  return { type: 'instant', scheduledAt: null, frequency_number: null, frequency_unit: null };
});

const checkScheduleWarning = (scheduledAt: string | null) => {
  if (!scheduledAt) {
    showScheduleWarning.value = false;
    return;
  }
  const scheduledTime = new Date(scheduledAt).getTime();
  const now = Date.now();
  const oneHourMs = 60 * 60 * 1000;
  showScheduleWarning.value = scheduledTime - now < oneHourMs;
};

const startEditing = () => {
  if (job.value) {
    const parsed = parsedJobType.value;
    editForm.value = {
      id: job.value.id,
      job_name: job.value.job_name,
      job_type: parsed.type,
      job_status: job.value.job_status,
      job_shell: job.value.job_shell,
      job_command: job.value.job_command,
      scheduled_at: parsed.scheduledAt,
      frequency_number: parsed.frequency_number ?? 1,
      frequency_unit: parsed.frequency_unit ?? 'hours'
    };
    selectedGroupIds.value = groups.value?.map(g => g.id) || [];
    checkScheduleWarning(parsed.scheduledAt);
    isEditing.value = true;
  }
};

const startManagingGroups = () => {
  managingGroupsGroupIds.value = groups.value?.map(g => g.id) || [];
  isManagingGroups.value = true;
};

const cancelManagingGroups = () => {
  isManagingGroups.value = false;
  managingGroupsGroupIds.value = [];
};

const saveManagedGroups = () => {
  updateGroupsMutation.mutate({ group_ids: managingGroupsGroupIds.value });
  isManagingGroups.value = false;
  managingGroupsGroupIds.value = [];
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

const deleteMutation = useMutation({
  mutationFn: async () => {
    const { error } = await deleteJob({ path: { id: jobId } });
    if (error) throw error;
  },
  onSuccess: () => {
    router.push('/jobs');
  }
});

const handleDelete = () => {
  deleteMutation.mutate();
};

const handleUpdate = () => {
  checkScheduleWarning(editForm.value.scheduled_at);

  let finalJobType: string;
  if (editForm.value.job_type === 'instant') {
    finalJobType = '"instant"';
  } else if (editForm.value.job_type === 'scheduled' && editForm.value.scheduled_at) {
    const schedWithSeconds = editForm.value.scheduled_at.length === 16
      ? editForm.value.scheduled_at + ':00'
      : editForm.value.scheduled_at;
    finalJobType = JSON.stringify({ scheduled: schedWithSeconds });
  } else if (editForm.value.job_type === 'recurring' && editForm.value.scheduled_at) {
    const frequencySecs = frequencyToSeconds(editForm.value.frequency_number, editForm.value.frequency_unit);
    const schedWithSeconds = editForm.value.scheduled_at.length === 16
      ? editForm.value.scheduled_at + ':00'
      : editForm.value.scheduled_at;
    finalJobType = JSON.stringify({ recurring: [schedWithSeconds, frequencySecs] });
  } else {
    finalJobType = '"instant"';
  }

  const { frequency_number, frequency_unit, ...rest } = editForm.value;
  const updatePayload = {
    ...rest,
    job_type: finalJobType
  };
  updateMutation.mutate(updatePayload as UpdateJobSrv);
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
        <div class="header-actions">
          <button v-if="job && !isEditing" @click="startEditing" class="btn-secondary">Edit
            Job</button>
          <button v-if="job && !isEditing" @click="showDeleteModal = true" class="btn-danger">Delete Job</button>
        </div>
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
                <option value="instant">Instant</option>
                <option value="scheduled">Scheduled</option>
                <option value="recurring">Recurring</option>
              </select>
            </div>
            <div class="form-group">
              <label>Status</label>
              <select v-model="editForm.job_status">
                <option value="pending">Pending</option>
                <option value="running">Running</option>
                <option value="paused">Paused</option>
                <option value="completed">Completed</option>
                <option value="failed">Failed</option>
                <option value="cancelled">Cancelled</option>
                <option value="timed_out">Timed Out</option>
                <option value="disabled">Disabled</option>
              </select>
            </div>
          </div>
          <div class="form-group">
            <label>Shell</label>
            <input type="text" v-model="editForm.job_shell" required />
          </div>
          <div class="form-group">
            <label>Command</label>
            <textarea v-model="editForm.job_command" rows="4" required></textarea>
          </div>

          <template v-if="editForm.job_type === 'scheduled' || editForm.job_type === 'recurring'">
            <div class="form-group">
              <label>Scheduled Time</label>
              <input type="datetime-local" v-model="editForm.scheduled_at" required />
              <span class="field-desc">Format: YYYY-MM-DDTHH:MM:SS</span>
            </div>
          </template>

          <template v-if="editForm.job_type === 'recurring'">
            <div class="form-group">
              <label>Repeat Every</label>
              <div class="form-row-inline">
                <input type="number" v-model="editForm.frequency_number" min="1" required />
                <select v-model="editForm.frequency_unit">
                  <option value="minutes">Minutes</option>
                  <option value="hours">Hours</option>
                  <option value="days">Days</option>
                  <option value="weeks">Weeks</option>
                </select>
              </div>
            </div>
          </template>

          <div v-if="showScheduleWarning" class="warning-banner">
            <strong>Warning:</strong> This job is scheduled less than 1 hour from now. The client may execute the job
            before this
            change takes effect.
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
              <span class="value">{{ parsedJobType.type }}</span>
            </div>
            <template v-if="parsedJobType.type !== 'instant'">
              <div class="info-item">
                <span class="label">Scheduled</span>
                <span class="value">{{ parsedJobType.scheduledAt }}</span>
              </div>
              <template v-if="parsedJobType.type === 'recurring'">
                <div class="info-item">
                  <span class="label">Frequency</span>
                  <span class="value">Every {{ parsedJobType.frequency_number }} {{ parsedJobType.frequency_unit
                    }}</span>
                </div>
              </template>
            </template>
            <div class="info-item">
              <span class="label">Created At</span>
              <span class="value">{{ formatDate(job.created_at) }}</span>
            </div>
            <div class="info-item">
              <span class="label">Updated At</span>
              <span class="value">{{ formatDate(job.updated_at) }}</span>
            </div>
          </div>
          <div class="info-item">
            <span class="label">Shell</span>
            <pre class="command-box">{{ job.job_shell }}</pre>
          </div>
          <div class="info-item full-width">
            <span class="label">Command</span>
            <pre class="command-box">{{ job.job_command }}</pre>
          </div>
        </div>
      </section>

      <!-- Group Assignment Section (shown when editing or managing groups) -->
      <section v-if="isEditing || isManagingGroups" class="info-section card">
        <div class="section-header">
          <div>
            <h2>Assigned Groups</h2>
            <a href="/groups/new" target="_blank" class="create-link">+ Create New Group</a>
          </div>
          <div class="header-actions-group">
            <div v-if="isManagingGroups" class="header-actions">
              <button @click="saveManagedGroups" class="btn-primary" :disabled="updateGroupsMutation.isPending.value">
                {{ updateGroupsMutation.isPending.value ? 'Saving...' : 'Save Groups' }}
              </button>
              <button @click="cancelManagingGroups" class="btn-ghost">Cancel</button>
            </div>
          </div>
        </div>
        <div class="group-selector">
          <div v-if="!allGroups || allGroups.length === 0" class="no-groups">
            No groups available. <a href="/groups/new" target="_blank">Create a group</a> first.
          </div>
          <div v-else v-for="group in allGroups" :key="group.id" class="group-checkbox">
            <input type="checkbox" :id="group.id" :value="group.id" v-model="activeGroupIds" />
            <label :for="group.id">
              <span class="group-name">{{ group.group_name }}</span>
              <span class="group-id">{{ group.id }}</span>
            </label>
          </div>
        </div>
      </section>

      <!-- Client Groups Section -->
      <section v-if="!isManagingGroups" class="client-tree-section card">
        <div class="section-header">
          <div>
            <h2>Client Groups</h2>
            <a href="/groups/new" target="_blank" class="create-link">+ Create New Group</a>
          </div>
          <div v-if="!isManagingGroups" class="header-actions">
            <button @click="startManagingGroups" class="btn-secondary">Manage Groups</button>
          </div>
        </div>
        <ClientTree v-if="!isEditing && !isManagingGroups" :groups="groupsWithClients || []" :job-id="jobId" />
      </section>
    </div>

    <ConfirmationModal v-if="showDeleteModal" title="Delete Job"
      message="Are you sure you want to delete this job? This action cannot be undone." confirm-text="Delete"
      variant="danger" @confirm="handleDelete" @cancel="showDeleteModal = false" />
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

.header-actions-group {
  display: flex;
  flex-direction: row;
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
  input[type="datetime-local"],
  select,
  textarea {
    padding: 0.6rem;
    border-radius: 0.4rem;
    border: 1px solid rgba(0, 0, 0, 0.1);
    background: var(--background-800);
    font-size: 0.95rem;
  }
}

.form-row-inline {
  display: flex;
  gap: 0.5rem;

  input,
  select {
    padding: 0.6rem;
    border-radius: 0.4rem;
    border: 1px solid rgba(0, 0, 0, 0.1);
    background: var(--background-800);
    font-size: 0.95rem;
    color: var(--text-950);
  }

  input[type="number"] {
    width: 80px;

    -webkit-inner-spin-button,
    -webkit-outer-spin-button {
      -webkit-appearance: none;
      margin: 0;
    }

    -moz-appearance: textfield;
  }

  select {
    flex: 1;
  }
}

input[type="datetime-local"] {
  padding: 0.6rem;
  border-radius: 0.4rem;
  border: 1px solid rgba(0, 0, 0, 0.1);
  background: var(--background-800);
  font-size: 0.95rem;
  color: var(--text-950);
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

.status-list {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}

.status-item {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 0.75rem 1rem;
  background: var(--background-200);
  border: 1px solid rgba(0, 0, 0, 0.1);
  border-radius: 0.5rem;

  .client-info {
    display: flex;
    flex-direction: column;
    gap: 0.125rem;

    .client-name {
      font-weight: 600;
      color: var(--text-900);
    }

    .client-id {
      font-size: 0.75rem;
      font-family: monospace;
      opacity: 0.5;
    }
  }

  .status-info {
    display: flex;
    align-items: center;
    gap: 1rem;

    .execution-count {
      font-size: 0.8rem;
      color: var(--text-500);
    }
  }

  .status-badge {
    display: inline-block;
    width: fit-content;
    padding: 0.2rem 0.6rem;
    border-radius: 9999px;
    font-weight: 600;
    font-size: 0.75rem;
    text-transform: capitalize;

    &.pending {
      background: var(--status-pending-bg);
      color: var(--status-pending-text);
    }

    &.running {
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
}

@keyframes spin {
  to {
    transform: rotate(360deg);
  }
}

.warning-banner {
  padding: 1rem;
  background-color: var(--warning-50, #fef3c7);
  border: 1px solid var(--warning-400, #fbbf24);
  color: var(--warning-900, #92400e);
  border-radius: 0.5rem;
  font-size: 0.9rem;
}

.field-desc {
  margin-top: 0.25rem;
  font-size: 0.8rem;
  opacity: 0.6;
  font-style: italic;
}
</style>
