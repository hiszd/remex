<script setup lang="ts">
import { ref, computed, h } from "vue"
import { useRoute, useRouter } from "vue-router"
import { useQuery, useMutation, useQueryClient } from "@tanstack/vue-query"
import { getGroupById, deleteGroup, updateGroup, getClients, getJobs, getJobsForGroup } from "@/lib/api"
import { useSurrealClient } from "@/lib/surreal"
import { formatDate } from "@/utils/date"
import IconJob from "@/components/icons/IconJob.vue"
import MultiSelector, { type selectItem } from "@/components/MultiSelector.vue"
import { toSelectItem } from "@/lib/utils"
import { extractEnumVariant, formatEnumVariant } from "@/lib/model"
import type { RecordId } from "surrealdb"

const route = useRoute()
const router = useRouter()
const queryClient = useQueryClient()
const client = useSurrealClient()
const groupId = route.params.id as string

const showDeleteModal = ref(false)

const { data: group, isLoading: loadingGroup, isError: groupError } = useQuery({
  queryKey: ["group", groupId],
  queryFn: () => getGroupById(client, groupId),
  select: (data) => Object.freeze(data),
})

const { data: allClients } = useQuery({
  queryKey: ["clients"],
  queryFn: () => getClients(client),
  select: (data) => Object.freeze(data),
})

const { data: allJobs } = useQuery({
  queryKey: ["jobs"],
  queryFn: () => getJobs(client),
})

const isEditingMembers = ref(false)

const selectedItems = computed(() => {
  if (!group.value || !allClients.value) return []

  const memberIds = new Set(group.value.members.map(m => String(m)))
  let sel = allClients.value.reduce((acc, c) => {
    if (memberIds.has(String(c.id))) {
      acc.push({
        id: c.id,
        display_name: c.client_name,
        item: c,
      })
    }
    return acc
  }, [] as selectItem[]);

  return sel ? sel : [];
})
const allItems = computed(() => {
  if (!group.value || !allClients.value) return []

  return allClients.value.map(c => ({
    id: c.id,
    name: c.client_name,
    type: 'client' as const,
  }))
})

const assignedJobs = computed(() => {
  if (!group.value || !allJobs.value) return []
  return getJobsForGroup(group.value.id, allJobs.value)
})

const jobItems = computed(() => {
  return assignedJobs.value.map(job => ({
    id: job.id,
    display_name: job.job_name,
    item: job,
    renderIcon: () => h(IconJob),
    renderContent: () => h('div', { class: 'assignment-badges' }, [
      h('span', {
        class: ['badge', 'state-badge', extractEnumVariant(job.enabled).toLowerCase()]
      }, extractEnumVariant(job.enabled)),
      h('span', {
        class: ['badge', 'status-badge', extractEnumVariant(job.execution_status).toLowerCase()]
      }, formatEnumVariant(extractEnumVariant(job.execution_status)))
    ])
  }))
})

const deleteMutation = useMutation({
  mutationFn: () => deleteGroup(client, groupId),
  onSuccess: () => {
    router.push("/groups")
  },
})

const updateMembersMutation = useMutation({
  mutationFn: (selectedIds: RecordId[]) =>
    updateGroup(client, groupId, { members: selectedIds }),
  onSuccess: () => {
    queryClient.invalidateQueries({ queryKey: ["group", groupId] })
    isEditingMembers.value = false
  },
})

const handleDelete = () => {
  deleteMutation.mutate()
}

const handleMembersSubmit = (selectedIds: RecordId[]) => {
  updateMembersMutation.mutate(selectedIds)
}

const handleMembersCancel = () => {
  isEditingMembers.value = false
}

const handleViewClientDetails = (item: selectItem) => {
  if (String(item.id.table) === 'client') {
    router.push(`/clients/${String(item.id)}`)
  }
}

const handleViewJobDetails = (item: selectItem) => {
  if (String(item.id.table) === 'job') {
    router.push(`/jobs/${String(item.id)}`)
  }
}
</script>

<template>
  <div class="page">
    <router-link to="/groups" class="back-link">← Back to Groups</router-link>

    <div v-if="group" class="header-main">
      <div class="title-group">
        <h1 class="page-title">{{ group.group_name }}</h1>
        <p class="record-id">{{ String(group.id) }}</p>
      </div>
      <div class="header-actions">
        <button @click="showDeleteModal = true" class="btn-danger">Delete Group</button>
      </div>
    </div>

    <div v-if="loadingGroup" class="state-card">
      <div class="spinner" />
      <p class="state-text">Loading group details...</p>
    </div>

    <div v-else-if="groupError || !group" class="state-card state-error">
      <p class="state-label">Group not found</p>
      <p class="state-text">The requested group could not be retrieved.</p>
    </div>

    <template v-else>
      <section class="card">
        <div class="section-header">
          <h2>General Information</h2>
        </div>

        <div class="info-grid">
          <div class="info-item">
            <span class="label">Group Name</span>
            <span class="value">{{ group.group_name }}</span>
          </div>
          <div class="info-item">
            <span class="label">ID</span>
            <span class="value monospace">{{ String(group.id) }}</span>
          </div>
          <div class="info-item">
            <span class="label">Created At</span>
            <span class="value">{{ formatDate(group.created_at) }}</span>
          </div>
        </div>
      </section>

      <section class="card">
        <div class="section-header">
          <h2>Members</h2>
          <button v-if="!isEditingMembers" @click="isEditingMembers = true" class="btn-secondary">Edit Members</button>
        </div>

        <MultiSelector :selectedItems="selectedItems" :allItems="allClients?.map(toSelectItem)"
          :mode="isEditingMembers ? 'edit' : 'view'" :showMembers="true" emptyText="No clients assigned"
          @submit="handleMembersSubmit" @cancel="handleMembersCancel" @viewDetails="handleViewClientDetails" />
      </section>

      <section class="card">
        <div class="section-header">
          <h2>Assigned Jobs ({{ assignedJobs.length }})</h2>
        </div>

        <MultiSelector :selectedItems="jobItems" :mode="'view'" :showMembers="true" emptyText="No jobs assigned"
          @viewDetails="handleViewJobDetails" />
      </section>
    </template>

    <div v-if="showDeleteModal" class="modal-overlay">
      <div class="modal-content">
        <h3>Delete Group</h3>
        <p>Are you sure you want to delete this group? This action cannot be undone.</p>
        <div class="modal-actions">
          <button @click="showDeleteModal = false" class="btn-ghost">Cancel</button>
          <button @click="handleDelete" class="btn-danger">Delete</button>
        </div>
      </div>
    </div>
  </div>
</template>

<style lang="scss" scoped>
.assignment-badges {
  display: flex;
  gap: 0.5rem;
}
</style>
