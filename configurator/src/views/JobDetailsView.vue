<script setup lang="ts">
import { ref, computed } from "vue"
import { useRoute, useRouter } from "vue-router"
import { useQuery, useMutation, useQueryClient } from "@tanstack/vue-query"
import { getJobById, updateJob, deleteJob } from "@/lib/api"
import { useSurrealClient } from "@/lib/surreal"
import { formatDate } from "@/utils/date"
import {
  extractEnumVariant,
  formatEnumVariant,
  makeEnabled,
  type EnabledVariant,
} from "@/lib/model"

const route = useRoute()
const router = useRouter()
const queryClient = useQueryClient()
const client = useSurrealClient()
const jobId = route.params.id as string

const showDeleteModal = ref(false)

const { data: job, isLoading: loadingJob, isError: jobError } = useQuery({
  queryKey: ["job", jobId],
  queryFn: () => getJobById(client, jobId),
})

const statusVariant = computed(() =>
  job.value ? extractEnumVariant(job.value.execution_status) : ""
)
const statusDisplay = computed(() => formatEnumVariant(statusVariant.value))
const enabledVariant = computed(() =>
  job.value ? extractEnumVariant(job.value.enabled) : ""
)

const isEditing = ref(false)
const editForm = ref({
  job_name: "",
  job_shell: "",
  job_command: "",
  job_type: "",
  enabled: "Draft" as EnabledVariant,
})

const startEditing = () => {
  if (job.value) {
    editForm.value = {
      job_name: job.value.job_name,
      job_shell: job.value.job_shell,
      job_command: job.value.job_command,
      job_type: extractEnumVariant(job.value.job_type),
      enabled: extractEnumVariant(job.value.enabled) as EnabledVariant,
    }
    isEditing.value = true
  }
}

const updateMutation = useMutation({
  mutationFn: (updatedJob: Partial<typeof editForm.value>) =>
    updateJob(client, jobId, {
      job_name: updatedJob.job_name,
      job_shell: updatedJob.job_shell,
      job_command: updatedJob.job_command,
      enabled: makeEnabled(updatedJob.enabled as EnabledVariant),
    }),
  onSuccess: () => {
    queryClient.invalidateQueries({ queryKey: ["job", jobId] })
    isEditing.value = false
  },
})

const deleteMutation = useMutation({
  mutationFn: () => deleteJob(client, jobId),
  onSuccess: () => {
    router.push("/jobs")
  },
})

const handleUpdate = () => {
  updateMutation.mutate(editForm.value)
}

const handleDelete = () => {
  deleteMutation.mutate()
}
</script>

<template>
  <div class="page">
    <header class="page-header">
      <router-link to="/jobs" class="back-link">← Back to Jobs</router-link>
      <div class="header-main">
        <div v-if="job" class="title-group">
          <h1 class="page-title">{{ job.job_name }}</h1>
          <p class="job-id">{{ String(job.id) }}</p>
        </div>
        <div class="header-actions">
          <button
            v-if="job && !isEditing"
            @click="startEditing"
            class="btn-secondary"
          >Edit Job</button>
          <button
            v-if="job && !isEditing"
            @click="showDeleteModal = true"
            class="btn-danger"
          >Delete Job</button>
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
      <section class="info-section card">
        <div class="section-header">
          <h2>General Information</h2>
        </div>

        <form
          v-if="isEditing"
          @submit.prevent="handleUpdate"
          class="edit-form"
        >
          <div class="form-group">
            <label>Name</label>
            <input v-model="editForm.job_name" type="text" required />
          </div>
          <div class="form-group">
            <label>Shell</label>
            <input type="text" v-model="editForm.job_shell" required />
          </div>
          <div class="form-group">
            <label>Command</label>
            <textarea
              v-model="editForm.job_command"
              rows="4"
              required
            ></textarea>
          </div>
          <div class="form-group">
            <label>State</label>
            <select v-model="editForm.enabled">
              <option value="Draft">Draft</option>
              <option value="Enabled">Enabled</option>
              <option value="Disabled">Disabled</option>
            </select>
          </div>
          <div class="form-actions">
            <button
              type="button"
              @click="isEditing = false"
              class="btn-ghost"
            >Cancel</button>
            <button
              type="submit"
              class="btn-primary"
              :disabled="updateMutation.isPending.value"
            >
              {{ updateMutation.isPending.value ? "Saving..." : "Save Changes" }}
            </button>
          </div>
        </form>

        <div v-else class="read-only-info">
          <div class="info-grid">
            <div class="info-item">
              <span class="label">Status</span>
              <span
                class="value status-badge"
                :class="statusVariant.toLowerCase()"
              >{{ statusDisplay }}</span>
            </div>
            <div class="info-item">
              <span class="label">State</span>
              <span class="value">{{ enabledVariant }}</span>
            </div>
            <div class="info-item">
              <span class="label">Type</span>
              <span class="value">{{ extractEnumVariant(job.job_type) }}</span>
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
          <div class="info-item">
            <span class="label">Shell</span>
            <pre class="command-box">{{ job.job_shell }}</pre>
          </div>
          <div class="info-item full-width">
            <span class="label">Command</span>
            <pre class="command-box">{{ job.job_command }}</pre>
          </div>
          <div v-if="job.assignments && job.assignments.length > 0" class="info-item full-width">
            <span class="label">Assignments</span>
            <div class="assignment-list">
              <span
                v-for="a in job.assignments"
                :key="String(a)"
                class="assignment-chip"
              >{{ String(a) }}</span>
            </div>
          </div>
        </div>
      </section>
    </div>

    <div v-if="showDeleteModal" class="modal-overlay">
      <div class="modal-content">
        <h3>Delete Job</h3>
        <p>Are you sure you want to delete this job? This action cannot be undone.</p>
        <div class="modal-actions">
          <button @click="showDeleteModal = false" class="btn-secondary">Cancel</button>
          <button @click="handleDelete" class="btn-danger">Delete</button>
        </div>
      </div>
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

  &.timedout {
    background: var(--status-timedout-bg);
    color: var(--status-timedout-text);
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

.assignment-list {
  display: flex;
  flex-wrap: wrap;
  gap: 0.5rem;
}

.assignment-chip {
  display: inline-flex;
  padding: 0.2rem 0.6rem;
  background: var(--background-200);
  border-radius: 0.375rem;
  font-size: 0.8rem;
  font-weight: 500;
  font-family: monospace;
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
  textarea,
  select {
    padding: 0.6rem;
    border-radius: 0.4rem;
    border: 1px solid rgba(0, 0, 0, 0.1);
    background: var(--background-800);
    font-size: 0.95rem;
    color: var(--text-950);
  }
}

.form-actions {
  display: flex;
  justify-content: flex-end;
  gap: 0.75rem;
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
}

.spinner {
  width: 2rem;
  height: 2rem;
  border: 3px solid rgba(0, 0, 0, 0.1);
  border-top-color: var(--accent-500);
  border-radius: 50%;
  animation: spin 0.8s linear infinite;
}

@keyframes spin {
  to {
    transform: rotate(360deg);
  }
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

.modal-overlay {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background: rgba(0, 0, 0, 0.5);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 1000;
}

.modal-content {
  background: var(--background-300);
  padding: 2rem;
  border-radius: 1rem;
  max-width: 400px;

  h3 {
    margin: 0 0 1rem;
  }

  p {
    margin: 0 0 1.5rem;
  }
}

.modal-actions {
  display: flex;
  justify-content: flex-end;
  gap: 0.75rem;
}
</style>
