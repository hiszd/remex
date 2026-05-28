<script setup lang="ts">
import { computed } from "vue"
import { formatDate } from "@/utils/date"
import { type Job, extractEnumVariant, formatEnumVariant } from "@/lib/model"
import { Hero } from "@/styles/color/hero"

const props = defineProps<{
  job: Job
  isExpanded?: boolean
}>()

defineEmits<{
  (e: "toggle-expand"): void
}>()

const created = formatDate(props.job.created_at)
const statusVariant = computed(() => extractEnumVariant(props.job.execution_status))
const statusDisplay = computed(() => formatEnumVariant(statusVariant.value))
const enabledVariant = computed(() => extractEnumVariant(props.job.enabled))
const typeVariant = computed(() => extractEnumVariant(props.job.job_type))
</script>

<template>
  <div class="job-card-container">
    <router-link :to="'/jobs/' + job.id" class="hero-card-link">
      <div class="hero-card" :style="[Hero]">
        <div class="hero-header">
          <h1 class="hero-name">{{ job.job_name }}</h1>
          <p class="hero-id">{{ String(job.id) }}</p>
        </div>
        <div class="hero-details">
          <span class="detail">
            <p class="label">Status</p>
            <p
              class="value status-badge"
              :class="statusVariant.toLowerCase()"
            >{{ statusDisplay }}</p>
          </span>
          <span class="detail">
            <p class="label">Type</p>
            <p class="value">{{ typeVariant }}</p>
          </span>
          <span class="detail">
            <p class="label">State</p>
            <p class="value">{{ enabledVariant }}</p>
          </span>
          <span class="detail full-width">
            <p class="label">Command</p>
            <p class="value monospace">{{ job.job_command }}</p>
          </span>
          <span class="detail full-width">
            <p class="label">Shell</p>
            <p class="value monospace">{{ job.job_shell }}</p>
          </span>
          <span class="detail">
            <p class="label">Created</p>
            <p class="value">{{ created }}</p>
          </span>
        </div>
      </div>
    </router-link>
  </div>
</template>

<style lang="scss" scoped>
@import "@/styles/hero-card.scss";

.job-card-container {
  display: flex;
  flex-direction: column;
}

.hero-card-link {
  padding: 0;
}

.hero-details .detail .value.status-badge {
  display: inline-block;
  width: fit-content;
  padding: 0.1rem 0.5rem;
  border-radius: 9999px;
  font-weight: 600;
  font-size: 0.8rem;
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
</style>
