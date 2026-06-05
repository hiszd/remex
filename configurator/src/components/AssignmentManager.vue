<script setup lang="ts">
import { ref, computed, type VNode, watch } from 'vue'
import type { RecordId } from 'surrealdb'
import type { Client } from '@/lib/model'
import IconGroup from '@/components/icons/IconGroup.vue'
import IconClient from '@/components/icons/IconClient.vue'
import IconJob from '@/components/icons/IconJob.vue'
import IconViewDetails from '@/components/icons/IconViewDetails.vue'

export interface AssignmentItem {
  id: RecordId
  name: string
  type: 'client' | 'group' | 'job'
  members?: Client[]
  renderIcon?: (item: AssignmentItem) => VNode
  renderContent?: (item: AssignmentItem) => VNode
}

const props = withDefaults(defineProps<{
  selectedItems: AssignmentItem[]
  allItems?: AssignmentItem[]
  showGroupMembers?: boolean
  emptyViewText?: string
  emptyEditText?: string
}>(), {
  showGroupMembers: false,
  emptyViewText: 'Nothing is currently assigned',
  emptyEditText: '',
})

const emit = defineEmits<{
  submit: [selectedIds: RecordId[]]
  cancel: []
  viewDetails: [item: AssignmentItem]
}>()

const localSelections = ref<Map<string, boolean>>(new Map())

const effectiveMode = computed(() => props.allItems ? 'edit' : 'view')
watch(() => props.selectedItems, (newItems) => {
  const newMap = new Map<string, boolean>()
  newItems.forEach(item => {
    newMap.set(String(item.id), true)
  })
  localSelections.value = newMap
}, { immediate: true })

const isSelected = (id: RecordId): boolean => {
  if (effectiveMode.value === 'view') {
    return props.selectedItems.find(item => String(item.id) === String(id)) ? true : false
  }
  return localSelections.value.get(String(id)) ?? false
}

const isMemberSelected = (clientId: RecordId): boolean => {
  if (!props.showGroupMembers) return false

  for (const item of props.selectedItems) {
    if (item.type === 'group' && item.members) {
      const groupSelected = effectiveMode.value === 'view'
        ? true
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
  if (effectiveMode.value !== 'edit') return

  const newSelected = !isSelected(item.id)
  localSelections.value.set(String(item.id), newSelected)
}

const handleSubmit = () => {
  const selectedIds: RecordId[] = []
  localSelections.value.forEach((selected, idStr) => {
    if (selected) {
      const item = props.allItems?.find(i => String(i.id) === idStr)
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
  if (effectiveMode.value === 'view') {
    return props.selectedItems
  }
  return props.allItems ? props.allItems : []
})

const hasItems = computed(() => displayItems.value.length > 0)
</script>

<template>
  <div class="assignment-manager">
    <div v-if="!hasItems" class="empty-state">
      <p class="empty-text">{{ effectiveMode === 'view' ? emptyViewText : emptyEditText }}</p>
    </div>

    <div v-else class="items-list">
      <div v-for="item in displayItems" :key="String(item.id)" class="item-wrapper">
        <div class="item" :class="{
          selected: isSelected(item.id),
          clickable: effectiveMode === 'edit'
        }" @click="toggleItem(item)">
          <div class="item-icon">
            <component v-if="item.renderIcon" :is="item.renderIcon(item)" />
            <IconGroup v-else-if="item.type === 'group'" />
            <IconClient v-else-if="item.type === 'client'" />
            <IconJob v-else-if="item.type === 'job'" />
          </div>
          <div v-if="item.name != ''" class="item-name">{{ item.name }}</div>
          <div v-if="item.name == ''" class="item-name">Nothing Here</div>
          <component v-if="item.renderContent" :is="item.renderContent(item)" class="item-content" />
          <button v-if="effectiveMode === 'view'" class="details-btn" @click.stop="handleViewDetails(item)"
            title="View details">
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

    <div v-if="effectiveMode === 'edit'" class="actions">
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

.item-content {
  display: flex;
  align-items: center;
  gap: 0.5rem;
}

.actions {
  display: flex;
  gap: 0.75rem;
  padding-top: 1rem;
  border-top: 1px solid var(--background-400);
}

.btn-secondary {
  margin-left: auto;
}
</style>
