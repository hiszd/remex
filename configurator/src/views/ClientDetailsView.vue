<script setup lang="ts">
import { computed } from "vue"
import { useRoute, useRouter } from "vue-router"
import { useQuery } from "@tanstack/vue-query"
import { getClientById, getJobs, getGroups, getJobsForClient, getGroupsForClient } from "@/lib/api"
import { useSurrealClient } from "@/lib/surreal"
import { formatDate } from "@/utils/date"
import { extractEnumVariant, formatEnumVariant } from "@/lib/model"
import IconViewDetails from "@/components/icons/IconViewDetails.vue"

const route = useRoute()
const router = useRouter()
const surreal = useSurrealClient()
const clientId = route.params.id as string

const { data: client, isLoading, isError } = useQuery({
  queryKey: ["client", clientId],
  queryFn: () => getClientById(surreal, clientId),
  select: (data) => Object.freeze(data),
})

const { data: jobs } = useQuery({
  queryKey: ["jobs"],
  queryFn: () => getJobs(surreal),
  select: (data) => Object.freeze(data),
})

const { data: groups } = useQuery({
  queryKey: ["groups"],
  queryFn: () => getGroups(surreal),
  select: (data) => Object.freeze(data),
})

const assignedJobs = computed(() => {
  if (!client.value || !jobs.value || !groups.value) return []
  return getJobsForClient(client.value.id, jobs.value, groups.value)
})

const assignedGroups = computed(() => {
  if (!client.value || !groups.value) return []
  return getGroupsForClient(client.value.id, groups.value)
})

const getStatusVariant = (job: any) => extractEnumVariant(job.enabled)
const getExecutionStatusVariant = (job: any) => extractEnumVariant(job.execution_status)
const getExecutionStatusDisplay = (job: any) => formatEnumVariant(getExecutionStatusVariant(job))

const navigateToJob = (jobId: any) => {
  router.push(`/jobs/${jobId}`)
}

const navigateToGroup = (groupId: any) => {
  router.push(`/groups/${groupId}`)
}
</script>

<template>
  <div class="page">
    <header class="page-header">
      <router-link to="/clients" class="back-link">← Back to Clients</router-link>
      <div v-if="client" class="header-main">
        <div class="title-group">
          <h1 class="page-title">{{ client.client_name }}</h1>
          <p class="client-id">{{ String(client.id) }}</p>
        </div>
      </div>
    </header>

    <div v-if="isLoading" class="state-card">
      <div class="spinner" />
      <p>Loading client details...</p>
    </div>

    <div v-else-if="isError || !client" class="state-card state-error">
      <p class="state-label">Client not found</p>
      <p class="state-text">The requested client could not be retrieved.</p>
    </div>

    <div v-else class="details-container">
      <section class="info-section card">
        <div class="section-header">
          <h2>General Information</h2>
        </div>

        <div class="info-grid">
          <div class="info-item">
            <span class="label">Client Name</span>
            <span class="value">{{ client.client_name }}</span>
          </div>
          <div class="info-item">
            <span class="label">ID</span>
            <span class="value monospace">{{ String(client.id) }}</span>
          </div>
          <div class="info-item">
            <span class="label">Hardware Hash</span>
            <span class="value monospace">{{ client.hardware_hash }}</span>
          </div>
          <div class="info-item">
            <span class="label">Last Seen</span>
            <span class="value">{{ client.last_seen ? formatDate(client.last_seen) : "Never" }}</span>
          </div>
          <div class="info-item">
            <span class="label">Created At</span>
            <span class="value">{{ formatDate(client.created_at) }}</span>
          </div>
        </div>
      </section>

      <section class="assignments-section card">
        <div class="section-header">
          <h2>Assigned Jobs ({{ assignedJobs.length }})</h2>
        </div>

        <div v-if="assignedJobs.length === 0" class="empty-state">
          <p>None assigned</p>
        </div>

        <div v-else class="assignment-list">
          <div
            v-for="job in assignedJobs"
            :key="String(job.id)"
            class="assignment-item"
          >
            <span class="assignment-name">{{ job.job_name }}</span>
            <div class="assignment-badges">
              <span class="badge state-badge" :class="getStatusVariant(job).toLowerCase()">
                {{ getStatusVariant(job) }}
              </span>
              <span class="badge status-badge" :class="getExecutionStatusVariant(job).toLowerCase()">
                {{ getExecutionStatusDisplay(job) }}
              </span>
            </div>
            <button class="details-btn" @click="navigateToJob(job.id)" title="View details">
              <IconViewDetails />
            </button>
          </div>
        </div>
      </section>

      <section class="assignments-section card">
        <div class="section-header">
          <h2>Assigned Groups ({{ assignedGroups.length }})</h2>
        </div>

        <div v-if="assignedGroups.length === 0" class="empty-state">
          <p>None assigned</p>
        </div>

        <div v-else class="assignment-list">
          <div
            v-for="group in assignedGroups"
            :key="String(group.id)"
            class="assignment-item"
          >
            <span class="assignment-name">{{ group.group_name }}</span>
            <button class="details-btn" @click="navigateToGroup(group.id)" title="View details">
              <IconViewDetails />
            </button>
          </div>
        </div>
      </section>
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

.assignments-section {
  .section-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }
}

.assignment-item {
  &:hover {
    background: var(--background-100);
  }
}
</style>
