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
})

const { data: jobs } = useQuery({
  queryKey: ["jobs"],
  queryFn: () => getJobs(surreal),
})

const { data: groups } = useQuery({
  queryKey: ["groups"],
  queryFn: () => getGroups(surreal),
})

const assignedJobs = computed(() => {
  if (!client.value || !jobs.value) return []
  return getJobsForClient(client.value.id, jobs.value)
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

  .client-id {
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

@keyframes spin {
  to {
    transform: rotate(360deg);
  }
}

.assignments-section {
  .section-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }
}

.empty-state {
  padding: 1.5rem;
  text-align: center;
  color: var(--text-500);
  font-size: 0.9rem;
}

.assignment-list {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}

.assignment-item {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 1rem;
  padding: 0.75rem 1rem;
  background: var(--background-200);
  border-radius: 0.5rem;
  transition: background 0.2s;
}

.details-btn {
  background: none;
  border: none;
  cursor: pointer;
  padding: 0.5rem;
  font-size: 1.25rem;
  opacity: 0.7;
  transition: opacity 0.2s;
  color: var(--text);
  flex-shrink: 0;

  &:hover {
    opacity: 1;
  }
}

.assignment-name {
  font-weight: 500;
}

.assignment-badges {
  display: flex;
  gap: 0.5rem;
}

.badge {
  display: inline-block;
  padding: 0.1rem 0.6rem;
  border-radius: 9999px;
  font-weight: 600;
  font-size: 0.75rem;
  background: var(--background-300);
}

.state-badge {
  &.draft {
    background: var(--status-pending-bg);
    color: var(--status-pending-text);
  }

  &.enabled {
    background: var(--status-completed-bg);
    color: var(--status-completed-text);
  }

  &.disabled {
    background: var(--status-failed-bg);
    color: var(--status-failed-text);
  }
}

.status-badge {
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

  &.timedout {
    background: var(--status-timedout-bg);
    color: var(--status-timedout-text);
  }
}
</style>
