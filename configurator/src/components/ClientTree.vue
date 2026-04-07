<script setup lang="ts">
import { ref } from 'vue'

interface GroupWithClients {
  id: string
  name: string
  client_ids: string[]
}

const props = defineProps<{
  groups: GroupWithClients[]
  jobId: string
}>()

const expandedGroups = ref<Set<string>>(new Set(props.groups.map(g => g.id)))

const toggleGroup = (groupId: string) => {
  if (expandedGroups.value.has(groupId)) {
    expandedGroups.value.delete(groupId)
  } else {
    expandedGroups.value.add(groupId)
  }
}

const isExpanded = (groupId: string) => expandedGroups.value.has(groupId)
</script>

<template>
  <div class="client-tree">
    <div v-if="!groups || groups.length === 0" class="tree-empty">
      No groups assigned to this job.
    </div>

    <div v-else class="tree-groups">
      <div v-for="group in groups" :key="group.id" class="tree-group">
        <div class="group-header" @click="toggleGroup(group.id)">
          <span class="expand-icon" :class="{ expanded: isExpanded(group.id) }">
            <svg width="12" height="12" viewBox="0 0 12 12" fill="currentColor">
              <path d="M4 2L8 6L4 10" stroke="currentColor" stroke-width="2" fill="none" />
            </svg>
          </span>
          <span class="group-name">{{ group.name }}</span>
          <span class="client-count">({{ group.client_ids.length }} {{ group.client_ids.length === 1 ? 'client' : 'clients' }})</span>
        </div>

        <div v-if="isExpanded(group.id)" class="group-clients">
          <div v-for="clientId in group.client_ids" :key="clientId" class="client-row">
            <span class="client-name">{{ clientId }}</span>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<style lang="scss" scoped>
.client-tree {
  width: 100%;
}

.tree-empty {
  padding: 1rem;
  color: var(--text-500);
  font-size: 0.9rem;
  font-style: italic;
}

.tree-groups {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}

.tree-group {
  border: 1px solid rgba(0, 0, 0, 0.1);
  border-radius: 0.5rem;
  overflow: hidden;
}

.group-header {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  padding: 0.75rem 1rem;
  background: var(--background-200);
  cursor: pointer;
  user-select: none;
  transition: background 0.2s;

  &:hover {
    background: var(--background-100);
  }
}

.expand-icon {
  display: flex;
  align-items: center;
  justify-content: center;
  transition: transform 0.2s;
  color: var(--text-500);

  &.expanded {
    transform: rotate(90deg);
  }
}

.group-name {
  font-weight: 600;
  color: var(--text-950);
}

.client-count {
  font-size: 0.8rem;
  color: var(--text-500);
}

.group-clients {
  background: var(--background-50);
  border-top: 1px solid var(--color-border);
}

.client-row {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 0.5rem 1rem 0.5rem 2rem;
  border-bottom: 1px solid rgba(0, 0, 0, 0.05);

  &:last-child {
    border-bottom: none;
  }
}

.client-name {
  font-size: 0.9rem;
  color: var(--text-900);
}
</style>
