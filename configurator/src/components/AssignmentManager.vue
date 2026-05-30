<script setup lang="ts">
import { ref, computed, watch } from 'vue'
import type { RecordId } from 'surrealdb'
import type { Client } from '@/lib/model'
import IconGroup from '@/components/icons/IconGroup.vue'
import IconClient from '@/components/icons/IconClient.vue'
import IconViewDetails from '@/components/icons/IconViewDetails.vue'

interface AssignmentItem {
  id: RecordId
  name: string
  type: 'client' | 'group'
  selected: boolean
  members?: Client[]
}

const props = withDefaults(defineProps<{
  mode: 'view' | 'edit'
  items: AssignmentItem[]
  showGroupMembers?: boolean
  emptyViewText?: string
  emptyEditText?: string
}>(), {
  showGroupMembers: false,
  emptyViewText: 'Nothing is currently assigned',
  emptyEditText: ''
})

const emit = defineEmits<{
  submit: [selectedIds: RecordId[]]
  cancel: []
  viewDetails: [item: AssignmentItem]
}>()

const localSelections = ref<Map<string, boolean>>(new Map())

watch(() => props.mode, (newMode) => {
  if (newMode === 'edit') {
    const newMap = new Map<string, boolean>()
    props.items.forEach(item => {
      newMap.set(String(item.id), item.selected)
    })
    localSelections.value = newMap
  }
}, { immediate: true })

const isSelected = (id: RecordId): boolean => {
  if (props.mode === 'view') {
    return props.items.find(item => String(item.id) === String(id))?.selected ?? false
  }
  return localSelections.value.get(String(id)) ?? false
}

const isMemberSelected = (clientId: RecordId): boolean => {
  if (!props.showGroupMembers) return false

  for (const item of props.items) {
    if (item.type === 'group' && item.members) {
      const groupSelected = props.mode === 'view'
        ? item.selected
        : (localSelections.value.get(String(item.id)) ?? false)

      if (groupSelected) {
        const isMember = item.members.some(member => String(member.id) === String(clientId))
        if (isMember) return true
      }
    }
  }

  return false
}

const toggleItem = (item: AssignmentItem) => {
  if (props.mode !== 'edit') return

  const newSelected = !isSelected(item.id)
  localSelections.value.set(String(item.id), newSelected)
}

const handleSubmit = () => {
  const selectedIds: RecordId[] = []
  localSelections.value.forEach((selected, idStr) => {
    if (selected) {
      const item = props.items.find(i => String(i.id) === idStr)
      if (item) {
        selectedIds.push(item.id)
      }
    }
  })
  emit('submit', selectedIds)
}

const handleCancel = () => {
  emit('cancel')
}

const handleViewDetails = (item: AssignmentItem) => {
  emit('viewDetails', item)
}

const displayItems = computed(() => {
  if (props.mode === 'view') {
    return props.items.filter(item => item.selected)
  }
  return props.items
})

const hasItems = computed(() => displayItems.value.length > 0)
</script>

<template>
  <div class="assignment-manager">
    <div v-if="!hasItems" class="empty-state">
      <p class="empty-text">{{ mode === 'view' ? emptyViewText : emptyEditText }}</p>
    </div>

    <div v-else class="items-list">
      <div v-for="item in displayItems" :key="String(item.id)" class="item-wrapper">
        <div class="item" :class="{
          selected: isSelected(item.id),
          clickable: mode === 'edit'
        }" @click="toggleItem(item)">
          <div class="item-icon">
            <IconGroup v-if="item.type === 'group'" />
            <IconClient v-else />
          </div>
          <div class="item-name">{{ item.name }}</div>
          <button v-if="mode === 'view'" class="details-btn" @click.stop="handleViewDetails(item)" title="View details">
            <IconViewDetails />
          </button>
        </div>

        <div v-if="showGroupMembers && item.type === 'group' && item.members" class="members-list">
          <div v-for="member in item.members" :key="String(member.id)" class="item member-item"
            :class="{ selected: isMemberSelected(member.id) }">
            <div class="item-icon">
              <IconClient />
            </div>
            <div class="item-name">{{ member.client_name }}</div>
          </div>
        </div>
      </div>
    </div>

    <div v-if="mode === 'edit'" class="actions">
      <button class="btn-primary" @click="handleSubmit">Submit</button>
      <button class="btn-secondary" @click="handleCancel">Cancel</button>
    </div>
  </div>
</template>

<style lang="scss" scoped>
.assignment-manager {
  display: flex;
  flex-direction: column;
  gap: 1rem;
}

.empty-state {
  padding: 2rem;
  text-align: center;
  background: var(--background-300);
  border-radius: 0.5rem;
}

.empty-text {
  margin: 0;
  color: var(--text-500);
  font-size: 0.9rem;
}

.items-list {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}

.item-wrapper {
  display: flex;
  flex-direction: column;
  gap: 0.25rem;
}

.item {
  display: flex;
  align-items: center;
  gap: 0.75rem;
  padding: 0.75rem 1rem;
  background: var(--background-300);
  border-radius: 0.5rem;
  transition: background-color 0.2s;

  &.clickable {
    cursor: pointer;

    &:hover {
      background: var(--background-200);
    }
  }

  &.selected {
    background: var(--primary-200);

    &.clickable:hover {
      background: var(--primary-300);
    }
  }
}

.member-item {
  margin-left: 2rem;
}

.item-icon {
  font-size: 1.25rem;
  flex-shrink: 0;
  color: var(--text);
}

.item-name {
  flex: 1;
  font-weight: 500;
}

.details-btn {
  color: var(--color-text);
  background: none;
  border: none;
  cursor: pointer;
  padding: 0.25rem;
  font-size: 1.25rem;
  opacity: 1;
  transition: opacity 0.2s;

  &:hover {
    opacity: 1;
  }
}

.actions {
  display: flex;
  gap: 0.75rem;
  padding-top: 1rem;
  border-top: 1px solid var(--background-400);
}

.btn-primary {
  padding: 0.6rem 1.25rem;
  border-radius: 0.5rem;
  background-color: var(--accent-500);
  color: white;
  font-weight: 700;
  border: none;
  cursor: pointer;
  transition: background 0.2s;

  &:hover {
    background-color: var(--accent-600);
  }
}

.btn-secondary {
  padding: 0.6rem 1.25rem;
  border-radius: 0.5rem;
  background-color: transparent;
  color: var(--text-950);
  font-weight: 600;
  border: 1px solid var(--background-400);
  cursor: pointer;
  margin-left: auto;
  transition: background 0.2s;

  &:hover {
    background-color: var(--background-200);
  }
}
</style>
