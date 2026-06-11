<script setup lang="ts">
import { ref } from "vue"
import { useRouter } from "vue-router"
import { useAuth } from "@/lib/auth"
import { useSurrealClient } from "@/lib/surreal"
import { setupGlobalInactivityListeners } from "@/lib/authGuard"

const router = useRouter()
const client = useSurrealClient()
const { login } = useAuth()

const email = ref("")
const password = ref("")
const isSubmitting = ref(false)
const errorMsg = ref<string | null>(null)

async function handleSubmit() {
  isSubmitting.value = true
  errorMsg.value = null
  try {
    await login(email.value, password.value)
    setupGlobalInactivityListeners(client, router)
    router.push("/")
  } catch (err: unknown) {
    errorMsg.value =
      err instanceof Error ? err.message : "Login failed. Check your credentials."
  } finally {
    isSubmitting.value = false
  }
}
</script>

<template>
  <div class="auth-card">
    <div class="auth-header">
      <h1 class="auth-title">remex</h1>
      <p class="auth-subtitle">Sign in to your account</p>
    </div>

    <form @submit.prevent="handleSubmit" class="auth-form">
      <div v-if="errorMsg" class="error-banner">{{ errorMsg }}</div>

      <div class="field">
        <label for="email" class="form-label">Email</label>
        <input
          id="email"
          v-model="email"
          type="email"
          class="form-input"
          placeholder="you@example.com"
          required
          autocomplete="email"
        />
      </div>

      <div class="field">
        <label for="password" class="form-label">Password</label>
        <input
          id="password"
          v-model="password"
          type="password"
          class="form-input"
          placeholder="Enter your password"
          required
          autocomplete="current-password"
        />
      </div>

      <button type="submit" class="btn-primary" :disabled="isSubmitting">
        {{ isSubmitting ? "Signing in..." : "Sign In" }}
      </button>
    </form>

    <p class="auth-footer">
      Don't have an account?
      <router-link to="/register">Create one</router-link>
    </p>
  </div>
</template>

<style lang="scss" scoped>
.auth-card {
  width: 100%;
  max-width: 400px;
  margin: 0 auto;
  padding: 2.5rem;
}

.field {
  display: flex;
  flex-direction: column;
  gap: 0.4rem;
}

.error-banner {
  padding: 0.75rem 1rem;
  border-radius: 0.5rem;
  background: var(--status-failed-bg);
  color: var(--status-failed-text);
  font-size: 0.85rem;
  font-weight: 600;
}
</style>
