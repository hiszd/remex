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
</template>

<style lang="scss" scoped>
.hero-card {
  border: 1px solid rgba(0 0 0 / 0);
  border-radius: 1rem;
  overflow: hidden;
  box-shadow: 0 4px 6px -1px rgba(0, 0, 0, 0.1), 0 2px 4px -1px rgba(0, 0, 0, 0.06);
}

.hero-header {
  padding: 1.5rem;
  border-bottom: 1px solid rgba(0 0 0 / 0.1);

  .hero-name {
    margin: 0;
    font-size: 2.25rem;
    font-weight: 800;
    line-height: 1.1;
    letter-spacing: -0.02em;
  }

  .hero-id {
    margin: 0.5rem 0 0;
    font-size: 0.85rem;
    font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, "Liberation Mono", "Courier New", monospace;
    opacity: 0.5;
  }
}

.hero-details {
  display: grid;
  grid-template-columns: 1fr;
  gap: 1.25rem;
  padding: 1.5rem;

  @media (min-width: 768px) {
    grid-template-columns: repeat(2, 1fr);
  }

  .detail {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;

    &.full-width {
      grid-column: span 2;
    }

    .label {
      margin: 0;
      font-size: 0.7rem;
      font-weight: 700;
      text-transform: uppercase;
      letter-spacing: 0.05em;
      opacity: 0.5;
    }

    .value {
      margin: 0;
      font-size: 0.95rem;
      line-height: 1.5;
      word-break: break-all;

      &.monospace {
        font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, "Liberation Mono", "Courier New", monospace;
        font-size: 0.85rem;
        background: rgba(0, 0, 0, 0.05);
        padding: 0.2rem 0.4rem;
        border-radius: 0.25rem;
      }
    }
  }
}
</style>
