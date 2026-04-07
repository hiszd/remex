<script setup lang="ts">
import { formatDate } from '@/utils/date';
import { Hero } from '@/styles/color/hero';
import type { Client } from '@/lib/model';

const props = defineProps<{
  client: Client
}>()

const created = formatDate(props.client.created_at)
</script>

<template>
  <router-link :to="'/clients/' + client.client_name" class="hero-card-link">
    <div class="hero-card" :style="[Hero]">
      <div class="hero-header">
        <h1 class="hero-name">{{ client.client_name }}</h1>
        <p class="hero-id">{{ client.client_name }}</p>
      </div>
      <div class="hero-details">
        <span class="detail">
          <p class="label">Created</p>
          <p class="value">{{ created }}</p>
        </span>
      </div>
    </div>
  </router-link>
</template>

<style lang="scss" scoped>
@import '@/styles/hero-card.scss';

.hero-card-link {
  padding: 0;
}

.hero-details .detail .value.monospace {
  background: rgba(0, 0, 0, 0.05);
}
</style>
