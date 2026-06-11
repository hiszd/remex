<script setup lang="ts">
defineProps<{
  isLoading: boolean
  isError: boolean
  error: unknown
  isEmpty: boolean
  emptyLabel: string
  emptySubtext: string
}>()
</script>

<template>
  <div v-if="isLoading" class="state-card">
    <div class="spinner" />
    <p class="state-text">Loading...</p>
  </div>
  <div v-else-if="isError" class="state-card state-error">
    <p class="state-label">Something went wrong</p>
    <p class="state-text">{{ error }}</p>
  </div>
  <div v-else-if="isEmpty" class="state-card state-empty">
    <p class="state-label">{{ emptyLabel }}</p>
    <p class="state-text">{{ emptySubtext }}</p>
  </div>
  <slot v-else />
</template>
