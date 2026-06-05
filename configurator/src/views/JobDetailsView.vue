<script setup lang="ts">
import { ref, computed } from "vue"
import { useRoute, useRouter } from "vue-router"
import { useQuery, useMutation, useQueryClient } from "@tanstack/vue-query"
import { getJobById, updateJob, deleteJob, getClients, getGroups, getClientById, getGroupById } from "@/lib/api"
import { useSurrealClient } from "@/lib/surreal"
import { formatDate } from "@/utils/date"
import {
  extractEnumVariant,
  formatEnumVariant,
  makeEnabled,
  type Client,
  type EnabledVariant,
} from "@/lib/model"
import AssignmentManager, { type AssignmentItem } from "@/components/AssignmentManager.vue"
import type { RecordId } from "surrealdb"

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

const getAssignments = computed(() => {
  let assignments: AssignmentItem[] = []
  for (const id of job.value?.assignments ?? []) {
    if (String(id).startsWith("client")) {
      const { data } = useQuery({
        queryKey: ["job", jobId],
        queryFn: () => getClientById(client, String(id)),
      });
      if (data?.value) {
        assignments.push(
          {
            id: data.value.id,
            name: data.value.client_name,
            type: 'client'
          }
        )
      } else {
        console.error(`Could not find client with id ${id}`)
      }
    } else {
      const { data } = useQuery({
        queryKey: ["job", jobId],
        queryFn: () => {
          try {
            return getGroupById(client, String(id))
          } catch (e) {
            console.error(e)
          }
        },
      });
      if (data?.value) {
        let members = []
        members = data.value.members?.reduce((acc, m) => {
          const { data: d } = useQuery({
            queryKey: ["job", jobId],
            queryFn: () => getClientById(client, String(m)),
          });
          if (d?.value) {
            acc.push(d.value)
          }
          return acc
        }, [] as Client[])
        if (members.length != data.value?.members?.length) {
          console.error(`Could not find members for group with id ${id}`)
          console.error(`${data.value.members} != ${members}`)
        }
        assignments.push({
          id: data.value.id,
          name: data.value.group_name,
          type: 'group',
          members,
        })
      } else {
        console.error(`Could not find group with id ${id}`)
      }
    }
  }
  console.log("assignments: ", assignments)
  return assignments;
});

const getAll = computed(() => {
  let all: AssignmentItem[] = [];
  const { data: clntdata } = useQuery({
    queryKey: ["jobClients", jobId],
    queryFn: () => getClients(client),
    enabled: isEditingAssignments,
  });
  if (clntdata?.value) {
    all.concat(clntdata?.value.map(c => {
      return {
        id: c.id,
        name: c.client_name,
        type: 'client'
      }
    }))
  }
  const { data: grpdata } = useQuery({
    queryKey: ["jobGroups", jobId],
    queryFn: () => getGroups(client),
    enabled: isEditingAssignments,
  });
  if (grpdata?.value) {
    all.concat(grpdata?.value.map(g => {
      return {
        id: g.id,
        name: g.group_name,
        type: 'group',
        members: g.members.reduce((acc, m) => {
          const d = clntdata?.value?.find(c => String(c.id) == String(m.id))
          if (d) {
            acc.push(d)
          }
          return acc
        }, [] as Client[])
      }
    }));
  }
  return all;
});

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

const isEditingAssignments = ref(false)

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

const handleViewDetails = (item: any) => {
  if (item.type === 'client') {
    router.push(`/clients/${item.id}`)
  } else if (item.type === 'group') {
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

        <AssignmentManager :selectedItems="getAssignments" :allItems="isEditingAssignments ? getAll : undefined"
          :show-group-members="true" empty-view-text="No clients or groups assigned"
          empty-edit-text="No clients or groups available" @submit="handleAssignmentSubmit"
          @cancel="handleAssignmentCancel" @view-details="handleViewDetails" />
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
