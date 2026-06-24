<script setup lang="ts">
import { ref, onMounted, onUnmounted, watch } from "vue"

const props = withDefaults(defineProps<{
  expanded?: boolean
  activeItem?: string
  username?: string
}>(), {
  expanded: true,
  activeItem: "/",
  username: "User",
})

const emit = defineEmits<{
  "update:expanded": [value: boolean]
  navigate: [path: string]
  logout: []
}>()

const isCollapsedMedia = ref(false)
let mql: MediaQueryList | null = null

function onMediaChange(e: MediaQueryListEvent | MediaQueryList) {
  if (e.matches) {
    isCollapsedMedia.value = true
    emit("update:expanded", false)
  }
}

onMounted(() => {
  mql = window.matchMedia("(max-width: 768px)")
  mql.addEventListener("change", onMediaChange)
  onMediaChange(mql)
})

onUnmounted(() => {
  mql?.removeEventListener("change", onMediaChange)
})

function toggle() {
  emit("update:expanded", !props.expanded)
}

const navItems = [
  { name: "Dashboard", path: "/design", icon: "dashboard" },
  { name: "Jobs", path: "/design/jobs", icon: "jobs" },
  { name: "Groups", path: "/design/groups", icon: "groups" },
  { name: "Clients", path: "/design/clients", icon: "clients" },
]


</script>

<template>
  <aside class="design-sidebar" :class="{ collapsed: !expanded }">
    <div class="sidebar-brand">
      <div class="brand-icon">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <circle cx="12" cy="12" r="10" />
          <path d="M12 6v12M6 12h12" />
        </svg>
      </div>
      <span v-if="expanded" class="brand-name">Remex</span>
    </div>

    <nav class="sidebar-nav">
      <button
        v-for="item in navItems"
        :key="item.path"
        class="nav-item"
        :class="{ active: activeItem === item.path }"
        @click="emit('navigate', item.path)"
        :title="item.name"
      >
        <!-- Dashboard / Home -->
        <svg v-if="item.icon === 'dashboard'" class="nav-icon" width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <path d="m3 9 9-7 9 7v11a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z" />
          <polyline points="9 22 9 12 15 12 15 22" />
        </svg>
        <!-- Jobs -->
        <svg v-else-if="item.icon === 'jobs'" class="nav-icon" width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <rect width="20" height="14" x="2" y="7" rx="2" ry="2" />
          <path d="M16 21V5a2 2 0 0 0-2-2h-4a2 2 0 0 0-2 2v16" />
        </svg>
        <!-- Groups -->
        <svg v-else-if="item.icon === 'groups'" class="nav-icon" width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <path d="M16 21v-2a4 4 0 0 0-4-4H6a4 4 0 0 0-4 4v2" />
          <circle cx="9" cy="7" r="4" />
          <path d="M22 21v-2a4 4 0 0 0-3-3.87" />
          <path d="M16 3.13a4 4 0 0 1 0 7.75" />
        </svg>
        <!-- Clients -->
        <svg v-else class="nav-icon" width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <rect width="14" height="8" x="5" y="2" rx="2" />
          <rect width="20" height="8" x="2" y="14" rx="2" />
          <path d="M6 18h2" />
          <path d="M12 18h6" />
        </svg>
        <span v-if="expanded" class="nav-label">{{ item.name }}</span>
      </button>
    </nav>

    <div class="sidebar-footer">
      <div class="user-section" :title="username">
        <div class="user-avatar-design">{{ username[0].toUpperCase() }}</div>
        <span v-if="expanded" class="user-name-design">{{ username }}</span>
      </div>
      <button class="logout-btn-design" @click="emit('logout')" title="Logout">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <path d="M9 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h4" />
          <polyline points="16 17 21 12 16 7" />
          <line x1="21" y1="12" x2="9" y2="12" />
        </svg>
        <span v-if="expanded">Logout</span>
      </button>
    </div>
  </aside>
</template>

<style scoped>
.design-sidebar {
  display: flex;
  flex-direction: column;
  width: 240px;
  min-width: 240px;
  height: 100vh;
  background: var(--sidebar-bg);
  color: var(--sidebar-text-muted);
  transition: width 0.2s ease, min-width 0.2s ease;
  overflow: hidden;

  &.collapsed {
    width: 60px;
    min-width: 60px;
  }
}

.sidebar-brand {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 0 1rem;
  height: 60px;
  border-bottom: 1px solid var(--sidebar-border);
  flex-shrink: 0;
  overflow: hidden;
}

.brand-icon {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 32px;
  height: 32px;
  border-radius: 0.5rem;
  background: var(--accent-600);
  color: white;
  flex-shrink: 0;

  svg {
    width: 18px;
    height: 18px;
  }
}

.brand-name {
  font-size: 1.125rem;
  font-weight: 800;
  color: var(--sidebar-text);
  white-space: nowrap;
}

.sidebar-nav {
  display: flex;
  flex-direction: column;
  gap: 2px;
  padding: 0.75rem 0.5rem;
  flex: 1;
  overflow-y: auto;
}

.nav-item {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 0.625rem 0.75rem;
  border-radius: 0.5rem;
  background: none;
  border: none;
  border-left: 3px solid transparent;
  color: var(--sidebar-text-muted);
  cursor: pointer;
  transition: all 0.15s;
  text-align: left;
  white-space: nowrap;
  font-size: 0.875rem;
  position: relative;

  &:hover {
    background: rgba(255, 255, 255, 0.08);
    color: var(--sidebar-text-hover);
  }

  &.active {
    background: var(--accent-600);
    color: white;
    border-left-color: white;
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.2);
  }
}

.collapsed .nav-item {
  padding: 0.625rem;
  justify-content: center;

  &.active {
    border-left-color: transparent;
    position: relative;

    &::before {
      content: "";
      position: absolute;
      left: 3px;
      top: 50%;
      transform: translateY(-50%);
      width: 3px;
      height: 20px;
      border-radius: 2px;
      background: white;
    }
  }
}

.nav-icon {
  width: 22px;
  height: 22px;
  flex-shrink: 0;
}

.nav-label {
  font-weight: 500;
}

.sidebar-footer {
  display: flex;
  flex-direction: column;
  gap: 2px;
  padding: 0.5rem;
  border-top: 1px solid var(--sidebar-border);
  flex-shrink: 0;
}

.user-section {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 0.5rem;
  border-radius: 0.5rem;
  overflow: hidden;
}

.user-avatar-design {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 28px;
  height: 28px;
  border-radius: 50%;
  background: var(--accent-600);
  color: white;
  font-size: 0.8rem;
  font-weight: 700;
  flex-shrink: 0;
}

.user-name-design {
  font-size: 0.85rem;
  font-weight: 600;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  color: var(--sidebar-text-muted);
}

.logout-btn-design {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 0.5rem;
  border-radius: 0.5rem;
  background: none;
  border: none;
  color: var(--sidebar-text-muted);
  cursor: pointer;
  transition: all 0.15s;
  white-space: nowrap;
  font-size: 0.85rem;

  &:hover {
    background: rgba(255, 255, 255, 0.08);
    color: var(--danger-text);
  }

  svg {
    width: 18px;
    height: 18px;
    flex-shrink: 0;
  }
}
</style>
