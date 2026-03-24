<script setup lang="ts">
import { formatDate } from '@/utils/date';
import { type Group, type ClientSrv } from '../client/index'
import { Hero } from '@/styles/color/hero';

const props = defineProps<{
  group: Group
  clients?: ClientSrv[]
}>()

const created = formatDate(props.group.created_at)
const updated = formatDate(props.group.updated_at)

const remainingClients = props.group.client_count - (props.clients?.length ?? 0)
</script>

<template>
  <router-link :to="'/groups/' + group.id" class="hero-card-link">
    <div class="hero-card" :style="[Hero]">
      <div class="hero-header">
        <div class="hero-header-top">
          <h1 class="hero-name">{{ group.group_name }}</h1>
          <span class="client-badge">{{ group.client_count }} {{ group.client_count === 1 ? 'client' : 'clients' }}</span>
        </div>
        <p class="hero-id">{{ group.id }}</p>
      </div>
      <div class="hero-details">
        <span class="detail">
          <p class="label">Created</p>
          <p class="value">{{ created }}</p>
        </span>
        <span class="detail">
          <p class="label">Updated</p>
          <p class="value">{{ updated }}</p>
        </span>
      </div>
      <div v-if="clients && clients.length > 0" class="hero-clients">
        <p class="label">Assigned Clients</p>
        <div class="client-list">
          <span v-for="client in clients" :key="client.id" class="client-chip">
            {{ client.client_name }}
          </span>
          <span v-if="remainingClients > 0" class="client-chip client-chip-more">
            +{{ remainingClients }} more
          </span>
        </div>
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
