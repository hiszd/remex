<script setup lang="ts">
import { ref, computed } from "vue"
import { useRoute, useRouter } from "vue-router"
import { useQuery, useMutation, useQueryClient } from "@tanstack/vue-query"
import { getJobById, updateJob, deleteJob, getClients, getGroups } from "@/lib/api"
import { useSurrealClient } from "@/lib/surreal"
import { formatDate } from "@/utils/date"
import {
  extractEnumVariant,
  formatEnumVariant,
  makeEnabled,
  type EnabledVariant,
} from "@/lib/model"
import MultiSelector, { type selectItem } from "@/components/MultiSelector.vue"
import type { RecordId } from "surrealdb"

const route = useRoute()
const router = useRouter()
const queryClient = useQueryClient()
const client = useSurrealClient()
const jobId = route.params.id as string

const showDeleteModal = ref(false)

const { data: job, isLoading: loadingJob, isError: jobError } = useQuery({
  queryKey: ["job", jobId],
  queryFn: async () => {
    const jb = await getJobById(client, jobId)
    if (!jb) throw new Error(`Could not find job with id ${jobId}`)
    return jb
  },
  select: (data) => Object.freeze(data),
  retry: 3,
})

const { data: allClients } = useQuery({
  queryKey: ["clients"],
  queryFn: () => getClients(client),
  select: (data) => Object.freeze(data),
})

const { data: allGroups } = useQuery({
  queryKey: ["groups"],
  queryFn: () => getGroups(client),
  select: (data) => Object.freeze(data),
})

const getAssignments = computed(() => {
  if (!job.value || !allClients.value || !allGroups.value) return []

  const assignments: selectItem[] = []
  for (const id of job.value.assignments ?? []) {
    const idStr = String(id)
    if (idStr.startsWith("client:")) {
      const found = allClients.value.find(c => String(c.id) === idStr)
      if (found) {
        assignments.push({ id: found.id, display_name: found.client_name, item: found })
      }
    } else if (idStr.startsWith("group:")) {
      const found = allGroups.value.find(g => String(g.id) === idStr)
      if (found) {
        const members = allClients.value.filter(c =>
          found.members.some(m => String(m) === String(c.id))
        )
        assignments.push({ id: found.id, display_name: found.group_name, item: found, members: members.map(m => { return { id: m.id, display_name: m.client_name, item: m } }) })
      }
    }
  }
  return assignments
})

const getAll = computed(() => {
  if (!allClients.value || !allGroups.value) return []

  const all: selectItem[] = []

  all.push(...allClients.value.map(c => ({
    id: c.id,
    display_name: c.client_name,
    item: c,
  })))

  all.push(...allGroups.value.map(g => {
    const members = allClients.value!.filter(c =>
      g.members.some(m => String(m) === String(c.id))
    )
    return {
      id: g.id,
      display_name: g.group_name,
      item: g,
      members: members.map(m => { return { id: m.id, display_name: m.client_name, item: m } }),
    }
  }))

  return all
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
  timeout: "",
  enabled: "Draft" as EnabledVariant,
})

const isEditingAssignments = ref(false)

const startEditing = () => {
  if (job.value) {
    editForm.value = {
      job_name: job.value.job_name,
      job_shell: job.value.job_shell,
      job_command: job.value.job_command,
      job_type: extractEnumVariant(job.value.job_type),
      timeout: job.value.timeout ?? "",
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
      timeout: updatedJob.timeout || null,
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

const updateAssignmentsMutation = useMutation({
  mutationFn: (selectedIds: RecordId[]) =>
    updateJob(client, jobId, { assignments: selectedIds }),
  onSuccess: () => {
    queryClient.invalidateQueries({ queryKey: ["job", jobId] })
    isEditingAssignments.value = false
  },
})

const handleUpdate = () => {
  updateMutation.mutate(editForm.value)
}

const handleDelete = () => {
  deleteMutation.mutate()
}

const handleAssignmentSubmit = (selectedIds: RecordId[]) => {
  updateAssignmentsMutation.mutate(selectedIds)
}

const handleAssignmentCancel = () => {
  isEditingAssignments.value = false
}

const handleViewAssignmentDetails = (item: selectItem) => {
  if (String(item.id.table) === 'client') {
    router.push(`/clients/${item.id}`)
  } else if (String(item.id.table) === 'group') {
    router.push(`/groups/${item.id}`)
  }
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
          <button v-if="job && !isEditing" @click="startEditing" class="btn-secondary">Edit Job</button>
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
      <section class="info-section card">
        <div class="section-header">
          <h2>General Information</h2>
        </div>

        <form v-if="isEditing" @submit.prevent="handleUpdate" class="edit-form">
          <div class="form-group">
            <label class="form-label">Name</label>
            <input v-model="editForm.job_name" type="text" class="form-input" required />
          </div>
          <div class="form-group">
            <label class="form-label">Shell</label>
            <input type="text" v-model="editForm.job_shell" class="form-input" required />
          </div>
          <div class="form-group">
            <label class="form-label">Command</label>
            <textarea v-model="editForm.job_command" class="form-input" rows="4" required></textarea>
          </div>
          <div class="form-group">
            <label class="form-label">Timeout (optional)</label>
            <input type="text" v-model="editForm.timeout" class="form-input" placeholder="e.g. 5m, 1h, 30s" />
          </div>
          <div class="form-group">
            <label class="form-label">State</label>
            <select v-model="editForm.enabled" class="form-input">
              <option value="Draft">Draft</option>
              <option value="Enabled">Enabled</option>
              <option value="Disabled">Disabled</option>
            </select>
          </div>
          <div class="form-actions">
            <button type="button" @click="isEditing = false" class="btn-ghost">Cancel</button>
            <button type="submit" class="btn-primary" :disabled="updateMutation.isPending.value">
              {{ updateMutation.isPending.value ? "Saving..." : "Save Changes" }}
            </button>
          </div>
        </form>

        <div v-else class="read-only-info">
          <div class="info-grid">
            <div class="info-item">
              <span class="label">Status</span>
              <span class="value status-badge" :class="statusVariant.toLowerCase()">{{ statusDisplay }}</span>
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
              <span class="label">Timeout</span>
              <span class="value">{{ job.timeout ?? "No timeout" }}</span>
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
        </div>
      </section>

      <section class="assignments-section card">
        <div class="section-header">
          <h2>Assignments</h2>
          <button v-if="!isEditingAssignments" @click="isEditingAssignments = true" class="btn-secondary">Edit
            Assignments</button>
        </div>

        <MultiSelector :selectedItems="getAssignments" :allItems="getAll" :mode="isEditingAssignments ? 'edit' : 'view'"
          :showMembers="true" emptyText="No clients or groups assigned" @submit="handleAssignmentSubmit"
          @cancel="handleAssignmentCancel" @viewDetails="handleViewAssignmentDetails" />
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
  width: 100%;
  max-width: 1000px;
  margin: 0 auto;
}

.details-container {
  display: flex;
  flex-direction: column;
  gap: 1.5rem;
}

.info-item {
  &.full-width {
    grid-column: 1 / -1;
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

.edit-form {
  display: flex;
  flex-direction: column;
  gap: 1rem;
}

.form-group {
  display: flex;
  flex-direction: column;
  gap: 0.4rem;
  margin-bottom: 0;

  textarea {
    resize: vertical;
    min-height: 100px;
  }

  select {
    cursor: pointer;
  }
}

.form-actions {
  display: flex;
  justify-content: flex-end;
  gap: 0.75rem;
  padding-top: 1rem;
  border-top: 1px solid var(--background-400);
}
</style>
