<script setup lang="ts">
import { ref } from "vue"
import { useRouter } from "vue-router"
import { useMutation, useQueryClient } from "@tanstack/vue-query"
import { createJob } from "@/lib/api"
import { useSurrealClient } from "@/lib/surreal"
import type { EnabledVariant } from "@/lib/model"

const router = useRouter()
const queryClient = useQueryClient()
const client = useSurrealClient()

const form = ref({
  job_name: "",
  job_shell: "/bin/bash",
  job_command: "",
  timeout: "",
  enabled: "Draft" as EnabledVariant,
})

const isSubmitting = ref(false)
const submitError = ref<string | null>(null)

const mutation = useMutation({
  mutationFn: (data: typeof form.value) =>
    createJob(client, {
      job_name: data.job_name,
      job_shell: data.job_shell,
      job_command: data.job_command,
      job_type: { Instant: {} },
      enabled: { [data.enabled]: {} } as Record<string, Record<string, never>>,
      assignments: [],
      timeout: data.timeout || null,
    }),
  onSuccess: () => {
    queryClient.invalidateQueries({ queryKey: ["jobs"] })
    router.push("/jobs")
  },
  onError: (err: Error) => {
    submitError.value = err.message || "Failed to create job"
    isSubmitting.value = false
  },
})

const handleSubmit = async () => {
  isSubmitting.value = true
  submitError.value = null
  mutation.mutate({
    job_name: form.value.job_name,
    job_shell: form.value.job_shell,
    job_command: form.value.job_command,
    enabled: form.value.enabled,
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
        <label for="job_name" class="form-label">Job Name</label>
        <input
          id="job_name"
          v-model="form.job_name"
          type="text"
          class="form-input"
          placeholder="e.g. System Update"
          required
        />
      </div>

      <div class="form-section">
        <label for="job_shell" class="form-label">Shell</label>
        <input
          type="text"
          id="job_shell"
          v-model="form.job_shell"
          class="form-input"
          placeholder="e.g. /bin/bash"
          required
        />
      </div>

      <div class="form-section">
        <label for="job_command" class="form-label">Command</label>
        <textarea
          id="job_command"
          v-model="form.job_command"
          class="form-input"
          placeholder="e.g. apt-get update"
          rows="4"
          required
        ></textarea>
      </div>

      <div class="form-section">
        <label for="timeout" class="form-label">Timeout (optional)</label>
        <input
          id="timeout"
          v-model="form.timeout"
          type="text"
          class="form-input"
          placeholder="e.g. 5m, 1h, 30s (leave empty for no timeout)"
        />
      </div>

      <div class="form-section">
        <label for="enabled" class="form-label">State</label>
        <select id="enabled" v-model="form.enabled" class="form-input">
          <option value="Draft">Draft</option>
          <option value="Enabled">Enabled</option>
          <option value="Disabled">Disabled</option>
        </select>
      </div>

      <div class="form-actions">
        <button
          type="button"
          @click="router.push('/jobs')"
          class="btn-secondary"
          :disabled="isSubmitting"
        >Cancel</button>
        <button type="submit" class="btn-primary" :disabled="isSubmitting">
          {{ isSubmitting ? "Creating..." : "Create Job" }}
        </button>
      </div>
    </form>
  </div>
</template>

<style lang="scss" scoped>
.page {
  width: 100%;
  max-width: 800px;
  margin: 0 auto;
}

.form-card {
  display: flex;
  flex-direction: column;
  gap: 1.5rem;
  padding: 2rem;
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

  textarea {
    resize: vertical;
    min-height: 100px;
  }

  select {
    cursor: pointer;
  }
}

.form-actions {
  display: flex;
  justify-content: flex-end;
  gap: 1rem;
  margin-top: 1rem;
  padding-top: 1.5rem;
  border-top: 1px solid var(--background-400);
}
</style>
