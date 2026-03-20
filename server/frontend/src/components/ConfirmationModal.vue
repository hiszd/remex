<script setup lang="ts">
defineProps<{
  title: string
  message: string
  confirmText?: string
  cancelText?: string
  variant?: 'default' | 'danger'
}>()

const emit = defineEmits<{
  (e: 'confirm'): void
  (e: 'cancel'): void
}>()

const handleOverlayClick = (event: MouseEvent) => {
  if (event.target === event.currentTarget) {
    emit('cancel')
  }
}
</script>

<template>
  <Teleport to="body">
    <div class="modal-overlay" @click="handleOverlayClick">
      <div class="modal-container" role="dialog" aria-modal="true">
        <div class="modal-header">
          <h2 class="modal-title">{{ title }}</h2>
        </div>
        <div class="modal-body">
          <p class="modal-message">{{ message }}</p>
        </div>
        <div class="modal-actions">
          <button type="button" class="btn-cancel" @click="emit('cancel')">
            {{ cancelText || 'Cancel' }}
          </button>
          <button
            type="button"
            class="btn-confirm"
            :class="{ 'btn-danger': variant === 'danger' }"
            @click="emit('confirm')"
          >
            {{ confirmText || 'Confirm' }}
          </button>
        </div>
      </div>
    </div>
  </Teleport>
</template>

<style lang="scss" scoped>
.modal-overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.6);
  backdrop-filter: blur(4px);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 9999;
  padding: 1rem;
}

.modal-container {
  background: var(--background-300);
  border-radius: 1rem;
  padding: 1.5rem;
  max-width: 400px;
  width: 100%;
  box-shadow: 0 20px 25px -5px rgba(0, 0, 0, 0.3), 0 10px 10px -5px rgba(0, 0, 0, 0.2);
}

.modal-header {
  margin-bottom: 1rem;
}

.modal-title {
  margin: 0;
  font-size: 1.25rem;
  font-weight: 700;
  color: var(--text-950);
}

.modal-body {
  margin-bottom: 1.5rem;
}

.modal-message {
  margin: 0;
  font-size: 0.95rem;
  color: var(--text-800);
  line-height: 1.5;
}

.modal-actions {
  display: flex;
  justify-content: flex-end;
  gap: 0.75rem;
}

.btn-cancel {
  background: transparent;
  border: 1px solid var(--text-500);
  color: var(--text-800);
  padding: 0.6rem 1.25rem;
  border-radius: 0.5rem;
  font-weight: 600;
  cursor: pointer;
  transition: all 0.15s ease;

  &:hover {
    background: var(--background-200);
  }
}

.btn-confirm {
  background: var(--accent-500);
  border: none;
  color: white;
  padding: 0.6rem 1.25rem;
  border-radius: 0.5rem;
  font-weight: 700;
  cursor: pointer;
  transition: all 0.15s ease;

  &:hover {
    background: var(--accent-600);
  }

  &.btn-danger {
    background: var(--danger-bg);
    color: var(--danger-text);
    border: 1px solid var(--danger-text);

    &:hover {
      opacity: 0.9;
    }
  }
}
</style>
