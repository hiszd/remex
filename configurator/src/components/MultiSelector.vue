<script setup lang="ts">
import { ref, computed, type VNode, watch } from 'vue'
import type { RecordId } from 'surrealdb'
import type { Client, Group, Job } from '@/lib/model'
import IconGroup from '@/components/icons/IconGroup.vue'
import IconClient from '@/components/icons/IconClient.vue'
import IconJob from '@/components/icons/IconJob.vue'
import IconViewDetails from '@/components/icons/IconViewDetails.vue'

export interface selectItem {
  id: RecordId
  display_name: string
  members?: selectItem[]
  item: Group | Client | Job
  renderIcon?: (item: selectItem) => VNode
  renderContent?: (item: selectItem) => VNode
}

const {
  mode,
  selectedItems,
  allItems,
  showMembers = false,
  emptyText = 'Nothing is currently assigned',
} = defineProps<{
  mode: 'view' | 'edit'
  selectedItems: selectItem[]
  allItems?: selectItem[]
  showMembers?: boolean
  emptyText?: string
}>()

const emit = defineEmits<{
  submit: [selectedIds: RecordId[]]
  cancel: []
  viewDetails: [item: selectItem]
  itemAdded: [item: selectItem]
}>()

const localSelections = ref<Map<string, boolean>>(new Map())

const handleSubmit = () => {
  const selectedIds: RecordId[] = []
  localSelections.value.forEach((selected, idStr) => {
    if (selected) {
      const item = allItems?.find(i => String(i.id) === idStr)
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

const handleViewDetails = (item: selectItem) => {
  emit('viewDetails', item)
}

const hasItems = computed(() => {
  let val = 0;
  if (allItems) {
    val += (allItems.length > 0) ? 1 : 0
  }
  val += (selectedItems.length > 0) ? 1 : 0
  return val > 0
})

watch(() => mode, () => {
  if (mode === 'view') {
    localSelections.value = new Map()
  } else {
    localSelections.value = new Map(selectedItems.map(item => [String(item.id), true]))
  }
})

const isSelected = (id: RecordId) => {
  if (mode === 'view') {
    return selectedItems.find(item => String(item.id) === String(id)) ? true : false
  } else {
    return localSelections.value.has(String(id))
  }
}

const toggleItem = (item: selectItem) => {
  if (mode === 'edit') {
    const newSelected = !isSelected(item.id)
    if (newSelected) {
      localSelections.value.set(String(item.id), newSelected)
      emit('itemAdded', item)
      return
    }
    localSelections.value.delete(String(item.id))
  }
}

const relevantItems = computed(() => {
  if (mode === 'view') {
    return selectedItems
  } else {
    return allItems ?? []
  }
})

</script>

<template>
  <div class="assignment-manager">
    <div v-if="!hasItems" class="empty-state">
      <p class="empty-text">{{ emptyText }}</p>
    </div>

    <div v-else class="items-list">
      <div v-for="item in relevantItems" :key="String(item.id)" class="item-wrapper">
        <div class="item" :class="{
          selected: isSelected(item.id),
          clickable: mode == 'edit'
        }" @click="toggleItem(item)">
          <div class="item-icon">
            <component v-if="item.renderIcon" :is="item.renderIcon(item)" />
            <IconGroup v-else-if="String(item.id.table) === 'group'" />
            <IconClient v-else-if="String(item.id.table) === 'client'" />
            <IconJob v-else-if="String(item.id.table) === 'job'" />
          </div>
          <div v-if="item.display_name != ''" class="item-name">{{ item.display_name }}</div>
          <div v-if="item.display_name == ''" class="item-name">Nothing Here</div>
          <component v-if="item.renderContent" :is="item.renderContent(item)" class="item-content" />
          <button v-if="mode === 'view'" class="details-btn" @click.stop="handleViewDetails(item)" title="View details">
            <IconViewDetails />
          </button>
        </div>

        <div v-if="showMembers && String(item.id.table) === 'group' && item.members" class="members-list">
          <div v-for="member in item.members" :key="String(member.id)" class="item member-item"
            :class="{ selected: isSelected(item.id) }">
            <div class="item-icon">
              <IconClient />
            </div>
            <div class="item-name">{{ member.display_name }}</div>
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
