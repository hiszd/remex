<script setup lang="ts">
import { formatDate } from '@/utils/date';
import { type Client } from '../client/index'
import { Hero } from '@/styles/color/hero';

const props = defineProps<{
  client: Client
}>()

const created = formatDate(props.client.created_at)
const updated = formatDate(props.client.updated_at)
</script>

<template>
  <router-link :to="'/clients/' + client.id" class="hero-card-link">
    <div class="hero-card" :style="[Hero]">
      <div class="hero-header">
        <h1 class="hero-name">{{ client.client_name }}</h1>
        <p class="hero-id">{{ client.id }}</p>
      </div>
      <div class="hero-details">
        <span class="detail full-width">
          <p class="label">Secret</p>
          <p class="value monospace">{{ client.secret }}</p>
        </span>
        <span class="detail full-width">
          <p class="label">Hardware Hash</p>
          <p class="value monospace">{{ client.hardware_hash }}</p>
        </span>
        <span class="detail">
          <p class="label">Created</p>
          <p class="value">{{ created }}</p>
        </span>
        <span class="detail">
          <p class="label">Updated</p>
          <p class="value">{{ updated }}</p>
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
