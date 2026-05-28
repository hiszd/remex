<script setup lang="ts">
import { ref, watch, onMounted } from "vue"
import { useRoute, useRouter } from "vue-router"
import { useAuth } from "@/lib/auth"

const route = useRoute()
const router = useRouter()
const isExpanded = ref(false)
const { currentUser, isAuthenticated, logout } = useAuth()

const STORAGE_KEY = "sidebar-expanded"

onMounted(() => {
  const stored = localStorage.getItem(STORAGE_KEY)
  isExpanded.value = stored === "true"
})

watch(isExpanded, (val) => {
  localStorage.setItem(STORAGE_KEY, String(val))
})

const navItems = [
  { name: "Home", path: "/", icon: "home" },
  { name: "Jobs", path: "/jobs", icon: "jobs" },
  { name: "Groups", path: "/groups", icon: "groups" },
  { name: "Clients", path: "/clients", icon: "clients" },
]

function isActive(path: string) {
  if (path === "/") {
    return route.path === "/"
  }
  return route.path.startsWith(path)
}

async function handleLogout() {
  await logout()
  router.push("/login")
}
</script>

<template>
  <aside class="sidebar" :class="{ expanded: isExpanded }">
    <button
      class="toggle-btn"
      @click="isExpanded = !isExpanded"
      :title="isExpanded ? 'Collapse' : 'Expand'"
    >
      <svg
        v-if="!isExpanded"
        xmlns="http://www.w3.org/2000/svg"
        width="20"
        height="20"
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        stroke-width="2"
        stroke-linecap="round"
        stroke-linejoin="round"
      >
        <line x1="3" y1="12" x2="21" y2="12" />
        <line x1="3" y1="6" x2="21" y2="6" />
        <line x1="3" y1="18" x2="21" y2="18" />
      </svg>
      <svg
        v-else
        xmlns="http://www.w3.org/2000/svg"
        width="20"
        height="20"
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        stroke-width="2"
        stroke-linecap="round"
        stroke-linejoin="round"
      >
        <polyline points="15 18 9 12 15 6" />
      </svg>
    </button>

    <nav class="nav-list">
      <router-link
        v-for="item in navItems"
        :key="item.path"
        :to="item.path"
        class="nav-item"
        :class="{ active: isActive(item.path) }"
        :title="item.name"
      >
        <svg
          v-if="item.icon === 'home'"
          xmlns="http://www.w3.org/2000/svg"
          width="22"
          height="22"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          stroke-width="2"
          stroke-linecap="round"
          stroke-linejoin="round"
        >
          <path d="m3 9 9-7 9 7v11a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z" />
          <polyline points="9 22 9 12 15 12 15 22" />
        </svg>
        <svg
          v-else-if="item.icon === 'jobs'"
          xmlns="http://www.w3.org/2000/svg"
          width="22"
          height="22"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          stroke-width="2"
          stroke-linecap="round"
          stroke-linejoin="round"
        >
          <rect width="20" height="14" x="2" y="7" rx="2" ry="2" />
          <path d="M16 21V5a2 2 0 0 0-2-2h-4a2 2 0 0 0-2 2v16" />
        </svg>
        <svg
          v-else-if="item.icon === 'groups'"
          xmlns="http://www.w3.org/2000/svg"
          width="22"
          height="22"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          stroke-width="2"
          stroke-linecap="round"
          stroke-linejoin="round"
        >
          <path d="M16 21v-2a4 4 0 0 0-4-4H6a4 4 0 0 0-4 4v2" />
          <circle cx="9" cy="7" r="4" />
          <path d="M22 21v-2a4 4 0 0 0-3-3.87" />
          <path d="M16 3.13a4 4 0 0 1 0 7.75" />
        </svg>
        <svg
          v-else-if="item.icon === 'clients'"
          xmlns="http://www.w3.org/2000/svg"
          width="22"
          height="22"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          stroke-width="2"
          stroke-linecap="round"
          stroke-linejoin="round"
        >
          <rect width="14" height="8" x="5" y="2" rx="2" />
          <rect width="20" height="8" x="2" y="14" rx="2" />
          <path d="M6 18h2" />
          <path d="M12 18h6" />
        </svg>
        <span v-if="isExpanded" class="nav-label">{{ item.name }}</span>
      </router-link>
    </nav>

    <div class="sidebar-footer" v-if="isAuthenticated">
      <div class="user-info" :title="currentUser?.email ?? ''">
        <div class="user-avatar">
          {{ (currentUser?.username ?? "U")[0].toUpperCase() }}
        </div>
        <span v-if="isExpanded" class="user-name">{{ currentUser?.username }}</span>
      </div>
      <button class="logout-btn" @click="handleLogout" :title="'Logout'">
        <svg
          xmlns="http://www.w3.org/2000/svg"
          width="18"
          height="18"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          stroke-width="2"
          stroke-linecap="round"
          stroke-linejoin="round"
        >
          <path d="M9 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h4" />
          <polyline points="16 17 21 12 16 7" />
          <line x1="21" y1="12" x2="9" y2="12" />
        </svg>
        <span v-if="isExpanded">Logout</span>
      </button>
    </div>
  </aside>
</template>

<style lang="scss" scoped>
.sidebar {
  display: flex;
  flex-direction: column;
  width: 60px;
  min-width: 60px;
  height: 100vh;
  background-color: var(--primary-800);
  color: var(--text-100);
  transition: width 0.2s ease, min-width 0.2s ease;
  overflow: hidden;

  &.expanded {
    width: 200px;
    min-width: 200px;
  }
}

.toggle-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 100%;
  height: 50px;
  background: none;
  border: none;
  color: var(--text-200);
  cursor: pointer;
  transition: background-color 0.15s ease, color 0.15s ease;
  flex-shrink: 0;

  &:hover {
    background-color: var(--primary-700);
    color: var(--text-50);
  }
}

.nav-list {
  display: flex;
  flex-direction: column;
  gap: 4px;
  padding: 8px;
  flex: 1;
}

.nav-item {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 12px;
  border-radius: 8px;
  color: var(--text-300);
  text-decoration: none;
  transition: background-color 0.15s ease, color 0.15s ease;
  white-space: nowrap;

  &:hover {
    background-color: var(--primary-700);
    color: var(--text-100);
  }

  &.active {
    background-color: var(--primary-600);
    color: var(--text-50);

    svg {
      color: var(--accent);
    }
  }

  svg {
    flex-shrink: 0;
  }
}

.nav-label {
  font-size: 14px;
  font-weight: 500;
}

.sidebar-footer {
  display: flex;
  flex-direction: column;
  gap: 4px;
  padding: 8px;
  border-top: 1px solid var(--primary-700);
}

.user-info {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 8px;
  border-radius: 8px;
  overflow: hidden;
}

.user-avatar {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 28px;
  height: 28px;
  border-radius: 50%;
  background-color: var(--accent-600);
  color: white;
  font-size: 0.8rem;
  font-weight: 700;
  flex-shrink: 0;
}

.user-name {
  font-size: 0.85rem;
  font-weight: 600;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.logout-btn {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 8px;
  border-radius: 8px;
  background: none;
  border: none;
  color: var(--text-400);
  cursor: pointer;
  transition: background-color 0.15s ease, color 0.15s ease;
  white-space: nowrap;
  font-size: 0.85rem;

  &:hover {
    background-color: var(--primary-700);
    color: var(--danger-text);
  }

  svg {
    flex-shrink: 0;
  }
}
</style>
