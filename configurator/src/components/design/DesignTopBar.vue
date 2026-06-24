<script setup lang="ts">
defineProps<{
  title: string
  breadcrumbs?: { label: string; path?: string }[]
  showSearch?: boolean
}>()

const emit = defineEmits<{
  toggleSidebar: []
  search: [query: string]
}>()
</script>

<template>
  <header class="top-bar">
    <div class="top-bar-left">
      <button class="hamburger-btn" @click="emit('toggleSidebar')" title="Toggle sidebar">
        <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <line x1="3" y1="12" x2="21" y2="12" />
          <line x1="3" y1="6" x2="21" y2="6" />
          <line x1="3" y1="18" x2="21" y2="18" />
        </svg>
      </button>

      <div v-if="breadcrumbs?.length" class="top-bar-breadcrumbs">
        <template v-for="(crumb, i) in breadcrumbs" :key="i">
          <span v-if="i > 0" class="separator">/</span>
          <router-link v-if="crumb.path" :to="crumb.path">{{ crumb.label }}</router-link>
          <span v-else class="current">{{ crumb.label }}</span>
        </template>
      </div>

      <span class="top-bar-title">{{ title }}</span>
    </div>

    <div class="top-bar-right">
      <div v-if="showSearch" class="search-input">
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <circle cx="11" cy="11" r="8" />
          <line x1="21" y1="21" x2="16.65" y2="16.65" />
        </svg>
        <input type="text" placeholder="Search..." @input="emit('search', ($event.target as HTMLInputElement).value)" />
      </div>

      <button class="hamburger-btn" title="Notifications">
        <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <path d="M18 8A6 6 0 0 0 6 8c0 7-3 9-3 9h18s-3-2-3-9" />
          <path d="M13.73 21a2 2 0 0 1-3.46 0" />
        </svg>
      </button>

      <div class="top-bar-avatar">
        <div class="user-avatar-small">U</div>
      </div>
    </div>
  </header>
</template>

<style scoped>
.top-bar-avatar {
  .user-avatar-small {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 32px;
    height: 32px;
    border-radius: 50%;
    background: var(--accent-600);
    color: white;
    font-size: 0.8rem;
    font-weight: 700;
    cursor: pointer;
  }
}
</style>
