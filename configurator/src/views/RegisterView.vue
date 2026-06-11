<script setup lang="ts">
import { ref } from "vue"
import { useRouter } from "vue-router"
import { useAuth } from "@/lib/auth"

const router = useRouter()
const { signup } = useAuth()

const username = ref("")
const email = ref("")
const password = ref("")
const isSubmitting = ref(false)
const errorMsg = ref<string | null>(null)

async function handleSubmit() {
  isSubmitting.value = true
  errorMsg.value = null
  try {
    await signup(username.value, email.value, password.value)
    router.push("/")
  } catch (err: unknown) {
    errorMsg.value =
      err instanceof Error ? err.message : "Registration failed."
  } finally {
    isSubmitting.value = false
  }
}
</script>

<template>
  <div class="auth-card">
    <div class="auth-header">
      <h1 class="auth-title">remex</h1>
      <p class="auth-subtitle">Create your account</p>
    </div>

    <form @submit.prevent="handleSubmit" class="auth-form">
      <div v-if="errorMsg" class="error-banner">{{ errorMsg }}</div>

      <div class="field">
        <label for="username" class="form-label">Username</label>
        <input
          id="username"
          v-model="username"
          type="text"
          class="form-input"
          placeholder="Choose a username"
          required
          autocomplete="username"
        />
      </div>

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
          placeholder="Choose a strong password"
          required
          autocomplete="new-password"
          minlength="8"
        />
      </div>

      <button type="submit" class="btn-primary" :disabled="isSubmitting">
        {{ isSubmitting ? "Creating account..." : "Create Account" }}
      </button>
    </form>

    <p class="auth-footer">
      Already have an account?
      <router-link to="/login">Sign in</router-link>
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
