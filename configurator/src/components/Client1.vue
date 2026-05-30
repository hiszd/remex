<script setup lang="ts">
import { useRouter } from 'vue-router';
import { formatDate } from '@/utils/date';
import { Hero } from '@/styles/color/hero';
import type { Client } from '@/lib/model';
import IconClient from '@/components/icons/IconClient.vue';
import IconViewDetails from '@/components/icons/IconViewDetails.vue';

const props = withDefaults(defineProps<{
  client: Client
  jobCount?: number
  groupCount?: number
}>(), {
  jobCount: 0,
  groupCount: 0
})

const router = useRouter();
const created = formatDate(props.client.created_at)

const navigateToDetails = () => {
  router.push(`/clients/${props.client.id}`);
}
</script>

<template>
  <div class="client-card" :style="[Hero]">
    <div class="item-icon">
      <IconClient />
    </div>
    <div class="client-info">
      <div class="hero-header">
        <h1 class="hero-name">{{ client.client_name }}</h1>
        <p class="hero-id">{{ client.client_name }}</p>
      </div>
      <div class="hero-details">
        <span class="detail">
          <p class="label">Jobs</p>
          <p class="value">{{ jobCount }}</p>
        </span>
        <span class="detail">
          <p class="label">Groups</p>
          <p class="value">{{ groupCount }}</p>
        </span>
        <span class="detail">
          <p class="label">Created</p>
          <p class="value">{{ created }}</p>
        </span>
      </div>
    </div>
    <button class="details-btn" @click="navigateToDetails" title="View details">
      <IconViewDetails />
    </button>
  </div>
</template>

<style lang="scss" scoped>
@import '@/styles/hero-card.scss';

.client-card {
  display: flex;
  align-items: center;
  gap: 1rem;
  padding: 1rem;
  background: var(--background-300);
  border-radius: 0.5rem;
  transition: background-color 0.2s;
}

.item-icon {
  font-size: 1.5rem;
  flex-shrink: 0;
  color: var(--text);
}

.client-info {
  flex: 1;
  min-width: 0;
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

  &:hover {
    opacity: 1;
  }
}

.hero-details .detail .value.monospace {
  background: rgba(0, 0, 0, 0.05);
}
</style>
