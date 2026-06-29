<script setup lang="ts">
import { computed } from "vue"
import { useRoute, useRouter } from "vue-router"
import { useQuery } from "@tanstack/vue-query"
import { getExecutionById, getJobById, getClientById } from "@/lib/api"
import { useSurrealClient } from "@/lib/surreal"
import { formatDate, formatDuration } from "@/utils/date"
import { extractEnumVariant, formatEnumVariant } from "@/lib/model"

const route = useRoute()
const router = useRouter()
const surreal = useSurrealClient()
const execId = route.params.id as string

const { data: execution, isLoading, isError } = useQuery({
  queryKey: ["execution", execId],
  queryFn: () => getExecutionById(surreal, execId),
  select: (data) => Object.freeze(data),
  retry: 3,
})

const execJobId = computed(() => execution.value?.job_id ? String(execution.value.job_id) : null)
const execClientId = computed(() => execution.value ? String(execution.value.client_id) : null)

const { data: job } = useQuery({
  queryKey: ["job-from-execution", execJobId],
  queryFn: () => {
    if (!execJobId.value) return null
    return getJobById(surreal, execJobId.value)
  },
  enabled: !!execJobId,
  select: (data) => Object.freeze(data),
})

const { data: client } = useQuery({
  queryKey: ["client-from-execution", execClientId],
  queryFn: () => {
    if (!execClientId.value) return null
    return getClientById(surreal, execClientId.value)
  },
  enabled: !!execClientId,
  select: (data) => Object.freeze(data),
})

const statusVariant = computed(() =>
  execution.value ? extractEnumVariant(execution.value.status) : ""
)
const statusDisplay = computed(() => formatEnumVariant(statusVariant.value))

const duration = computed(() => {
  if (!execution.value?.execution_start || !execution.value?.execution_end) return null
  return formatDuration(
    new Date(execution.value.execution_start),
    new Date(execution.value.execution_end),
  )
})

const navigateToJob = () => {
  if (execution.value?.job_id) {
    router.push(`/jobs/${execution.value.job_id}`)
  }
}

const navigateToClient = () => {
  if (execution.value) {
    router.push(`/clients/${execution.value.client_id}`)
  }
}
</script>

<template>
  <div class="page">
    <router-link to="/jobs" class="back-link">← Back to Jobs</router-link>

    <div v-if="execution" class="header-main">
      <div class="title-group">
        <h1 class="page-title">Execution Details</h1>
        <p class="record-id">{{ String(execution.id) }}</p>
      </div>
    </div>

    <div v-if="isLoading" class="state-card">
      <div class="spinner" />
      <p class="state-text">Loading execution details...</p>
    </div>

    <div v-else-if="isError || !execution" class="state-card state-error">
      <p class="state-label">Execution not found</p>
      <p class="state-text">The requested execution could not be retrieved.</p>
    </div>

    <template v-else>
      <section class="card">
        <div class="section-header">
          <h2>General Information</h2>
        </div>

        <div class="info-grid">
          <div class="info-item">
            <span class="label">Status</span>
            <span class="value status-badge" :class="statusVariant.toLowerCase()">{{ statusDisplay }}</span>
          </div>
          <div class="info-item">
            <span class="label">Client</span>
            <span class="value">
              <a v-if="client" href="#" @click.prevent="navigateToClient" class="link">{{ client.client_name }}</a>
              <span v-else class="monospace">{{ String(execution.client_id) }}</span>
            </span>
          </div>
          <div class="info-item">
            <span class="label">Job</span>
            <span class="value">
              <a v-if="job" href="#" @click.prevent="navigateToJob" class="link">{{ job.job_name }}</a>
              <span v-else-if="execution.job_id" class="monospace">{{ String(execution.job_id) }}</span>
              <span v-else>-</span>
            </span>
          </div>
          <div class="info-item">
            <span class="label">Exit Code</span>
            <span class="value monospace">{{ execution.exit_code || "-" }}</span>
          </div>
          <div class="info-item">
            <span class="label">Started</span>
            <span class="value">{{ execution.execution_start ? formatDate(execution.execution_start) : "-" }}</span>
          </div>
          <div class="info-item">
            <span class="label">Ended</span>
            <span class="value">{{ execution.execution_end ? formatDate(execution.execution_end) : "-" }}</span>
          </div>
          <div class="info-item">
            <span class="label">Duration</span>
            <span class="value">{{ duration ?? "-" }}</span>
          </div>
          <div class="info-item">
            <span class="label">Created At</span>
            <span class="value">{{ formatDate(execution.created_at) }}</span>
          </div>
          <div class="info-item">
            <span class="label">Updated At</span>
            <span class="value">{{ formatDate(execution.updated_at) }}</span>
          </div>
        </div>
      </section>

      <section class="card">
        <div class="section-header">
          <h2>Command</h2>
        </div>
        <pre class="output-box">{{ execution.command || "-" }}</pre>
      </section>

      <section class="card">
        <div class="section-header">
          <h2>Output</h2>
        </div>
        <pre v-if="execution.output" class="output-box">{{ execution.output }}</pre>
        <p v-else class="empty-state" style="margin: 0;">No output</p>
      </section>
    </template>
  </div>
</template>

<style lang="scss" scoped>
.output-box {
  background: var(--background-100);
  padding: 1rem;
  border-radius: 0.5rem;
  font-family: ui-monospace, monospace;
  font-size: 0.85rem;
  overflow-x: auto;
  white-space: pre-wrap;
  word-break: break-all;
  margin: 0;
  max-height: 400px;
  overflow-y: auto;
}

.link {
  color: var(--accent);
  text-decoration: none;
  font-weight: 600;

  &:hover {
    text-decoration: underline;
  }
}
</style>
