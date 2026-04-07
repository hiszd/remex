<script setup lang="ts">
import { formatDate } from '@/utils/date';
import { type Group, type Client } from '@/lib/api'
import { Hero } from '@/styles/color/hero';

const props = defineProps<{
  group: Group
  clients?: Client[]
}>()

const created = formatDate(props.group.created_at)
</script>

<template>
  <router-link :to="'/groups/' + group.id" class="hero-card-link">
    <div class="hero-card" :style="[Hero]">
      <div class="hero-header">
        <h1 class="hero-name">{{ group.name }}</h1>
        <p class="hero-id">{{ group.id }}</p>
      </div>
      <div class="hero-details">
        <span class="detail">
          <p class="label">Created</p>
          <p class="value">{{ created }}</p>
        </span>
        <span class="detail">
          <p class="label">Client IDs</p>
          <p class="value">{{ group.client_ids?.length ?? 0 }} clients</p>
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
</style>
