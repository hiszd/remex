<script setup lang="ts">
import { formatDate } from '@/utils/date';
import { type Job } from '@/lib/api'
import { Hero } from '@/styles/color/hero';

const _props = defineProps<{
  job: Job
  isExpanded?: boolean
}>()

defineEmits<{
  (e: 'toggle-expand'): void
}>()

const created = formatDate(_props.job.created_at)
</script>

<template>
  <div class="job-card-container">
    <router-link :to="'/jobs/' + job.id" class="hero-card-link">
      <div class="hero-card" :style="[Hero]">
        <div class="hero-header">
          <h1 class="hero-name">{{ job.name }}</h1>
          <p class="hero-id">{{ job.id }}</p>
        </div>
        <div class="hero-details">
          <span class="detail">
            <p class="label">Status</p>
            <p class="value status-badge" :class="job.status.toLowerCase()">{{ job.status }}</p>
          </span>
          <span class="detail">
            <p class="label">Type</p>
            <p class="value">{{ job.job_type }}</p>
          </span>
          <span class="detail full-width">
            <p class="label">Command</p>
            <p class="value monospace">{{ job.command }}</p>
          </span>
          <span class="detail full-width">
            <p class="label">Shell</p>
            <p class="value monospace">{{ job.shell }}</p>
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
@import '@/styles/hero-card.scss';

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

  &.failed {
    background: var(--status-failed-bg);
    color: var(--status-failed-text);
  }
}
</style>
