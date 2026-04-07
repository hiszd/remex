<script setup lang="ts">
import { ref } from 'vue'
import { useRouter } from 'vue-router'
import { useMutation, useQueryClient } from '@tanstack/vue-query'
import { createJob } from '@/lib/api'

const router = useRouter()
const queryClient = useQueryClient()

const form = ref({
  name: '',
  description: '',
  job_type: 'instant',
  shell: '/bin/bash',
  command: '',
  group_ids: [] as string[],
})

const isSubmitting = ref(false)
const submitError = ref<string | null>(null)

const mutation = useMutation({
  mutationFn: createJob,
  onSuccess: () => {
    queryClient.invalidateQueries({ queryKey: ['jobs'] })
    router.push('/jobs')
  },
  onError: (err: Error) => {
    submitError.value = err.message || 'Failed to create job'
    isSubmitting.value = false
  }
})

const handleSubmit = async () => {
  isSubmitting.value = true
  submitError.value = null
  mutation.mutate({
    name: form.value.name,
    description: form.value.description,
    job_type: form.value.job_type,
    shell: form.value.shell,
    command: form.value.command,
    group_ids: form.value.group_ids,
  })
}
</script>

<template>
  <div class="page">
    <header class="page-header">
      <router-link to="/jobs" class="back-link">← Back to Jobs</router-link>
      <h1 class="page-title">Create New Job</h1>
      <p class="page-subtitle">Configure a new task to be executed by your clients.</p>
    </header>

    <form @submit.prevent="handleSubmit" class="form-card">
      <div v-if="submitError" class="error-banner">
        {{ submitError }}
      </div>

      <div class="form-section">
        <label for="name">Job Name</label>
        <input id="name" v-model="form.name" type="text" placeholder="e.g. System Update" required />
      </div>

      <div class="form-section">
        <label for="job_type">Type</label>
        <select id="job_type" v-model="form.job_type">
          <option value="instant">Instant</option>
          <option value="scheduled">Scheduled</option>
          <option value="recurring">Recurring</option>
        </select>
      </div>

      <div class="form-section">
        <label for="shell">Shell</label>
        <input type="text" id="shell" v-model="form.shell" placeholder="e.g. /bin/bash" required />
      </div>

      <div class="form-section">
        <label for="command">Command</label>
        <textarea id="command" v-model="form.command" placeholder="e.g. apt-get update" rows="4" required></textarea>
      </div>

      <div class="form-actions">
        <button type="button" @click="router.push('/jobs')" class="btn-secondary" :disabled="isSubmitting">
          Cancel
        </button>
        <button type="submit" class="btn-primary" :disabled="isSubmitting">
          {{ isSubmitting ? 'Creating...' : 'Create Job' }}
        </button>
      </div>
    </form>
  </div>
</template>

<style lang="scss" scoped>
.page {
  display: flex;
  flex-direction: column;
  width: 100%;
  max-width: 800px;
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

.page-header {
  .page-title {
    margin: 0;
    font-size: 1.75rem;
    font-weight: 800;
  }

  .page-subtitle {
    margin: 0.25rem 0 0;
    font-size: 0.95rem;
    color: var(--text-500);
  }
}

.form-card {
  display: flex;
  flex-direction: column;
  gap: 1.5rem;
  padding: 2rem;
  border-radius: 1rem;
  background-color: var(--background-300);
  color: var(--text-950);
  box-shadow: 0 4px 6px -1px rgba(0, 0, 0, 0.1);
}

.error-banner {
  padding: 1rem;
  background-color: var(--accent-50);
  border: 1px solid var(--accent-400);
  color: var(--accent-900);
  border-radius: 0.5rem;
  font-size: 0.9rem;
}

.form-section {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;

  label {
    font-size: 0.85rem;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.025em;
    opacity: 0.8;
  }

  input[type="text"],
  select,
  textarea {
    padding: 0.75rem 1rem;
    border-radius: 0.5rem;
    border: 1px solid var(--color-border);
    background: var(--background-50);
    font-size: 1rem;
    color: var(--text-950);

    &:focus {
      outline: none;
      border-color: var(--accent-500);
      box-shadow: 0 0 0 2px var(--accent-100);
    }
  }
}

.form-actions {
  display: flex;
  justify-content: flex-end;
  gap: 1rem;
  margin-top: 1rem;
  padding-top: 1.5rem;
  border-top: 1px solid rgba(0, 0, 0, 0.1);
}

.btn-primary {
  padding: 0.75rem 2rem;
  border-radius: 0.5rem;
  background-color: var(--accent-500);
  color: white;
  font-weight: 700;
  border: none;
  cursor: pointer;
  transition: background 0.2s;

  &:hover:not(:disabled) {
    background-color: var(--accent-600);
  }

  &:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
}

.btn-secondary {
  padding: 0.75rem 2rem;
  border-radius: 0.5rem;
  background-color: transparent;
  color: var(--text-950);
  font-weight: 600;
  border: 1px solid rgba(0, 0, 0, 0.1);
  cursor: pointer;

  &:hover:not(:disabled) {
    background-color: rgba(0, 0, 0, 0.05);
  }
}
</style>
