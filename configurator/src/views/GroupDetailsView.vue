<script setup lang="ts">
import { ref } from "vue"
import { useRoute, useRouter } from "vue-router"
import { useQuery, useMutation, useQueryClient } from "@tanstack/vue-query"
import { getGroupById, deleteGroup } from "@/lib/api"
import { useSurrealClient } from "@/lib/surreal"
import { formatDate } from "@/utils/date"

const route = useRoute()
const router = useRouter()
const queryClient = useQueryClient()
const client = useSurrealClient()
const groupId = route.params.id as string

const showDeleteModal = ref(false)

const { data: group, isLoading: loadingGroup, isError: groupError } = useQuery({
  queryKey: ["group", groupId],
  queryFn: () => getGroupById(client, groupId),
})

const deleteMutation = useMutation({
  mutationFn: () => deleteGroup(client, groupId),
  onSuccess: () => {
    router.push("/groups")
  },
})

const handleDelete = () => {
  deleteMutation.mutate()
}
</script>

<template>
  <div class="page">
    <header class="page-header">
      <router-link to="/groups" class="back-link">← Back to Groups</router-link>
      <div v-if="group" class="header-main">
        <div class="title-group">
          <h1 class="page-title">{{ group.group_name }}</h1>
          <p class="group-id">{{ String(group.id) }}</p>
        </div>
        <div class="header-actions">
          <button @click="showDeleteModal = true" class="btn-danger">Delete Group</button>
        </div>
      </div>
    </header>

    <div v-if="loadingGroup" class="state-card">
      <div class="spinner" />
      <p>Loading group details...</p>
    </div>

    <div v-else-if="groupError || !group" class="state-card state-error">
      <p class="state-label">Group not found</p>
      <p class="state-text">The requested group could not be retrieved.</p>
    </div>

    <div v-else class="details-container">
      <section class="info-section card">
        <div class="section-header">
          <h2>General Information</h2>
        </div>

        <div class="info-grid">
          <div class="info-item">
            <span class="label">Group Name</span>
            <span class="value">{{ group.group_name }}</span>
          </div>
          <div class="info-item">
            <span class="label">ID</span>
            <span class="value monospace">{{ String(group.id) }}</span>
          </div>
          <div class="info-item">
            <span class="label">Created At</span>
            <span class="value">{{ formatDate(group.created_at) }}</span>
          </div>
        </div>
      </section>

      <section
        v-if="group.members && group.members.length > 0"
        class="members-section card"
      >
        <div class="section-header">
          <h2>Members ({{ group.members.length }})</h2>
        </div>

        <div class="member-list">
          <router-link
            v-for="member in group.members"
            :key="String(member)"
            :to="'/clients/' + String(member)"
            class="member-chip"
          >
            {{ String(member) }}
          </router-link>
        </div>
      </section>
    </div>

    <div v-if="showDeleteModal" class="modal-overlay">
      <div class="modal-content">
        <h3>Delete Group</h3>
        <p>Are you sure you want to delete this group? This action cannot be undone.</p>
        <div class="modal-actions">
          <button @click="showDeleteModal = false" class="btn-secondary">Cancel</button>
          <button @click="handleDelete" class="btn-danger">Delete</button>
        </div>
      </div>
    </div>
  </div>
</template>

<style lang="scss" scoped>
.page {
  display: flex;
  flex-direction: column;
  width: 100%;
  max-width: 1000px;
  margin: 0 auto;
  padding: 2rem 1.5rem;
  gap: 2rem;
}

.back-link {
  display: inline-block;
  margin-bottom: 1rem;
  font-size: 0.9rem;
  color: var(--accent-500);
  text-decoration: none;

  &:hover {
    text-decoration: underline;
  }
}

.header-main {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  gap: 1rem;
}

.header-actions {
  display: flex;
  gap: 0.75rem;
}

.title-group {
  .page-title {
    margin: 0;
    font-size: 2rem;
    font-weight: 800;
  }

  .group-id {
    margin: 0.25rem 0 0;
    font-size: 0.9rem;
    font-family: monospace;
    opacity: 0.5;
  }
}

.details-container {
  display: flex;
  flex-direction: column;
  gap: 1.5rem;
}

.card {
  background: var(--background-300);
  color: var(--text-950);
  border-radius: 1rem;
  padding: 1.5rem;
  box-shadow: 0 4px 6px -1px rgba(0, 0, 0, 0.1);
}

.section-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 1.5rem;
  border-bottom: 1px solid rgba(0, 0, 0, 0.1);
  padding-bottom: 0.75rem;

  h2 {
    margin: 0;
    font-size: 1.25rem;
    font-weight: 700;
  }
}

.info-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
  gap: 1.5rem;
}

.info-item {
  display: flex;
  flex-direction: column;
  gap: 0.25rem;

  .label {
    font-size: 0.7rem;
    font-weight: 700;
    text-transform: uppercase;
    opacity: 0.6;
  }

  .value {
    font-size: 1rem;

    &.monospace {
      font-family: ui-monospace, monospace;
      font-size: 0.85rem;
      background: rgba(0, 0, 0, 0.05);
      padding: 0.2rem 0.4rem;
      border-radius: 0.25rem;
      word-break: break-all;
    }
  }
}

.member-list {
  display: flex;
  flex-wrap: wrap;
  gap: 0.5rem;
}

.member-chip {
  display: inline-flex;
  padding: 0.4rem 0.8rem;
  background: var(--background-200);
  border: 1px solid rgba(0, 0, 0, 0.1);
  border-radius: 0.5rem;
  text-decoration: none;
  color: inherit;
  font-family: monospace;
  font-size: 0.85rem;
  transition: background 0.2s, border-color 0.2s;

  &:hover {
    background: var(--background-100);
    border-color: var(--accent-400);
  }
}

.state-card {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 0.75rem;
  padding: 3rem 1.5rem;
  border-radius: 1rem;
  background-color: var(--background-300);
  color: var(--text-950);
  text-align: center;
}

.state-label {
  margin: 0;
  font-size: 1.1rem;
  font-weight: 700;
}

.state-text {
  margin: 0;
  font-size: 0.9rem;
  opacity: 0.7;
}

.state-error {
  border: 1px solid var(--accent-400);

  .state-label {
    color: var(--accent-500);
  }
}

.spinner {
  width: 2rem;
  height: 2rem;
  border: 3px solid rgba(0, 0, 0, 0.1);
  border-top-color: var(--accent-500);
  border-radius: 50%;
  animation: spin 0.8s linear infinite;
}

@keyframes spin {
  to {
    transform: rotate(360deg);
  }
}

.btn-danger {
  background: var(--danger-bg);
  color: var(--danger-text);
  border: 1px solid var(--danger-text);
  padding: 0.6rem 1.25rem;
  border-radius: 0.5rem;
  font-weight: 600;
  cursor: pointer;
}

.modal-overlay {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background: rgba(0, 0, 0, 0.5);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 1000;
}

.modal-content {
  background: var(--background-300);
  padding: 2rem;
  border-radius: 1rem;
  max-width: 400px;

  h3 {
    margin: 0 0 1rem;
  }

  p {
    margin: 0 0 1.5rem;
  }
}

.modal-actions {
  display: flex;
  justify-content: flex-end;
  gap: 0.75rem;
}
</style>
