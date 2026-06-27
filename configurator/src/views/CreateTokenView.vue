<script setup lang="ts">
import { ref, computed } from "vue"
import { useRouter } from "vue-router"
import { useMutation, useQueryClient } from "@tanstack/vue-query"
import { createEnrollmentToken } from "@/lib/api"
import { useSurrealClient } from "@/lib/surreal"

const router = useRouter()
const queryClient = useQueryClient()
const client = useSurrealClient()

const singleUse = ref(true)
const hasExpiration = ref(false)
const expiresDate = ref("")

const plaintextToken = ref<string | null>(null)
const copySuccess = ref(false)
const submitError = ref<string | null>(null)
const isSubmitting = ref(false)

const expiresAt = computed(() => {
  if (!hasExpiration.value || !expiresDate.value) return null
  return new Date(expiresDate.value).toISOString()
})

const mutation = useMutation({
  mutationFn: () =>
    createEnrollmentToken(client, {
      single_use: singleUse.value,
      expires_at: expiresAt.value,
    }),
  onSuccess: (result) => {
    plaintextToken.value = result.plaintext_token
    isSubmitting.value = false
  },
  onError: (err: Error) => {
    submitError.value = err.message || "Failed to generate token"
    isSubmitting.value = false
  },
})

async function handleSubmit() {
  isSubmitting.value = true
  submitError.value = null
  mutation.mutate()
}

async function copyToken() {
  if (!plaintextToken.value) return
  try {
    await navigator.clipboard.writeText(plaintextToken.value)
    copySuccess.value = true
    setTimeout(() => { copySuccess.value = false }, 2000)
  } catch {
    copySuccess.value = false
  }
}

function closeTokenDisplay() {
  queryClient.invalidateQueries({ queryKey: ["enrollment-tokens"] })
  router.push("/tokens")
}
</script>

<template>
  <div class="page" style="max-width: 800px;">
    <router-link to="/tokens" class="back-link">← Back to Tokens</router-link>
    <h1 class="page-title" style="margin-bottom: 1.5rem;">Generate Enrollment Token</h1>

    <!-- Token display modal -->
    <div v-if="plaintextToken" class="modal-overlay" @click.self="closeTokenDisplay">
      <div class="modal-content token-display-modal">
        <div class="success-icon">
          <svg width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M22 11.08V12a10 10 0 1 1-5.93-9.14" />
            <polyline points="22 4 12 14.01 9 11.01" />
          </svg>
        </div>
        <h2 style="margin: 0 0 0.25rem 0;">Token Generated</h2>
        <p class="page-subtitle" style="margin-bottom: 1.5rem;">
          Copy this token now. You won't be able to see it again.
        </p>

        <div class="token-display">
          <code class="token-text">{{ plaintextToken }}</code>
        </div>

        <div class="modal-actions" style="margin-top: 1.5rem;">
          <button class="btn-primary" @click="copyToken">
            <template v-if="copySuccess">
              <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" style="margin-right: 4px;">
                <polyline points="20 6 9 17 4 12" />
              </svg>
              Copied!
            </template>
            <template v-else>
              <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" style="margin-right: 4px;">
                <rect x="9" y="9" width="13" height="13" rx="2" ry="2" />
                <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1" />
              </svg>
              Copy Token
            </template>
          </button>
          <button class="btn-ghost" @click="closeTokenDisplay">Done</button>
        </div>
      </div>
    </div>

    <!-- Form -->
    <form @submit.prevent="handleSubmit" class="card">
      <div v-if="submitError" class="error-banner">
        {{ submitError }}
      </div>

      <div class="form-group">
        <label class="form-label">Usage Type</label>
        <div class="radio-group">
          <label class="radio-item">
            <input type="radio" v-model="singleUse" :value="true" />
            <div>
              <strong>Single Use</strong>
              <span class="radio-desc">Token is invalidated after one endpoint enrolls</span>
            </div>
          </label>
          <label class="radio-item">
            <input type="radio" v-model="singleUse" :value="false" />
            <div>
              <strong>Multi Use</strong>
              <span class="radio-desc">Token can be used by multiple endpoints</span>
            </div>
          </label>
        </div>
      </div>

      <div class="form-group">
        <label class="form-label">Expiration</label>
        <div class="radio-group">
          <label class="radio-item">
            <input type="radio" v-model="hasExpiration" :value="false" />
            <div>
              <strong>Never</strong>
              <span class="radio-desc">Token never expires</span>
            </div>
          </label>
          <label class="radio-item">
            <input type="radio" v-model="hasExpiration" :value="true" />
            <div>
              <strong>Custom</strong>
              <span class="radio-desc">Token expires at a specific date and time</span>
            </div>
          </label>
        </div>
        <input
          v-if="hasExpiration"
          type="datetime-local"
          v-model="expiresDate"
          class="form-input"
          style="margin-top: 0.5rem;"
          required
        />
      </div>

      <div class="form-actions">
        <button type="button" @click="router.push('/tokens')" class="btn-ghost" :disabled="isSubmitting">Cancel</button>
        <button type="submit" class="btn-primary" :disabled="isSubmitting">
          {{ isSubmitting ? "Generating..." : "Generate Token" }}
        </button>
      </div>
    </form>
  </div>
</template>

<style lang="scss" scoped>
.form-group {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}

.radio-group {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}

.radio-item {
  display: flex;
  align-items: flex-start;
  gap: 0.75rem;
  padding: 0.75rem;
  border: 1px solid var(--background-400);
  border-radius: 0.5rem;
  cursor: pointer;
  transition: border-color 0.15s;

  &:hover {
    border-color: var(--accent-400);
  }

  input[type="radio"] {
    margin-top: 0.2rem;
    accent-color: var(--accent-600);
  }

  strong {
    display: block;
    font-size: 0.9rem;
    color: var(--text-900);
  }
}

.radio-desc {
  display: block;
  font-size: 0.8rem;
  color: var(--text-500);
  margin-top: 0.15rem;
}

.error-banner {
  padding: 1rem;
  background-color: var(--accent-50);
  border: 1px solid var(--accent-400);
  color: var(--accent-900);
  border-radius: 0.5rem;
  font-size: 0.9rem;
}

.form-actions {
  display: flex;
  justify-content: flex-end;
  gap: 1rem;
  margin-top: 1rem;
  padding-top: 1.5rem;
  border-top: 1px solid var(--background-400);
}

.token-display-modal {
  text-align: center;
  max-width: 480px;
}

.success-icon {
  color: var(--green-500, #22c55e);
  margin-bottom: 0.5rem;
}

.token-display {
  background: var(--background-200);
  border: 1px solid var(--background-400);
  border-radius: 0.5rem;
  padding: 1rem;
  overflow-x: auto;
}

.token-text {
  font-family: "SF Mono", "Fira Code", "Fira Mono", monospace;
  font-size: 1.1rem;
  letter-spacing: 0.05em;
  word-break: break-all;
  color: var(--text-900);
  user-select: all;
}

.modal-actions {
  display: flex;
  justify-content: center;
  gap: 0.75rem;
}
</style>
