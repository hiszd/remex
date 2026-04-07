<script setup lang="ts">
import { formatDate } from '@/utils/date';
import { type Client } from '@/lib/api'
import { Hero } from '@/styles/color/hero';
const props = defineProps<{
  client: Client
}>()

const created = formatDate(props.client.created_at)
</script>

<template>
  <div class="client-card" :style=[Hero]>
    <h2>{{ client.name }}</h2>
    <div class="data">
      <span class="line">
        <p class="label">ID: </p>
        <p class="value">{{ client.id }}</p>
      </span>
      <span class="line">
        <p class="label">Created: </p>
        <p class="value">{{ created }}</p>
      </span>
    </div>
  </div>
</template>

<style lang="scss" scoped>
.client-card {
  border: 1px solid rgba(0 0 0 / 0);
  border-radius: 1rem;
  margin-bottom: 10px;

  .line {
    display: flex;
    flex-direction: row;
    justify-content: space-between;

    .label {
      font-weight: bold;
      margin-right: 1rem;
    }

    .value {}
  }

  .data {
    display: flex;
    flex-direction: column;
    gap: 1em;
    padding: 0.5rem;
  }

  h2 {
    margin: 0;
    padding: 0.1em 0.5em;
    font-weight: bold;
    border-bottom: 0.1em solid rgba(0 0 0 / 1);
  }
}
</style>
