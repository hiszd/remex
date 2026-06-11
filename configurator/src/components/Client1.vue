<script setup lang="ts">
import { formatDate } from "@/utils/date"
import { Hero } from "@/styles/color/hero"
import type { Client } from "@/lib/model"

const { client, jobCount = 0, groupCount = 0 } = withDefaults(defineProps<{
  client: Client
  jobCount?: number
  groupCount?: number
}>(), {
  jobCount: 0,
  groupCount: 0
})

const created = formatDate(client.created_at)
const lastSeen = client.last_seen ? formatDate(client.last_seen) : "Never"
</script>

<template>
  <div class="client-card-container">
    <router-link :to="'/clients/' + client.id" class="hero-card-link">
      <div class="hero-card" :style="[Hero]">
        <div class="hero-header">
          <h1 class="hero-name">{{ client.client_name }}</h1>
          <p class="hero-id">{{ String(client.id) }}</p>
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
            <p class="label">Last Seen</p>
            <p class="value">{{ lastSeen }}</p>
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

.client-card-container {
  display: flex;
  flex-direction: column;
}

.hero-card-link {
  padding: 0;
}
</style>
