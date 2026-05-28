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
        <label for="username">Username</label>
        <input
          id="username"
          v-model="username"
          type="text"
          placeholder="Choose a username"
          required
          autocomplete="username"
        />
      </div>

      <div class="field">
        <label for="email">Email</label>
        <input
          id="email"
          v-model="email"
          type="email"
          placeholder="you@example.com"
          required
          autocomplete="email"
        />
      </div>

      <div class="field">
        <label for="password">Password</label>
        <input
          id="password"
          v-model="password"
          type="password"
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
  padding: 2.5rem;
  border-radius: 1rem;
  background: var(--background-300);
  box-shadow: 0 8px 24px rgba(0, 0, 0, 0.12);
}

.auth-header {
  text-align: center;
  margin-bottom: 2rem;
}

.auth-title {
  margin: 0;
  font-size: 2rem;
  font-weight: 800;
  color: var(--accent-500);
}

.auth-subtitle {
  margin: 0.5rem 0 0;
  font-size: 0.95rem;
  color: var(--text-500);
}

.auth-form {
  display: flex;
  flex-direction: column;
  gap: 1.25rem;
}

.field {
  display: flex;
  flex-direction: column;
  gap: 0.4rem;

  label {
    font-size: 0.8rem;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.025em;
    color: var(--text-600);
  }

  input {
    padding: 0.75rem 1rem;
    border-radius: 0.5rem;
    border: 1px solid var(--background-400);
    background: var(--background-50);
    font-size: 1rem;
    color: var(--text-950);
    transition: border-color 0.15s;

    &:focus {
      outline: none;
      border-color: var(--accent-500);
      box-shadow: 0 0 0 2px var(--accent-100);
    }

    &::placeholder {
      color: var(--text-400);
    }
  }
}

.error-banner {
  padding: 0.75rem 1rem;
  border-radius: 0.5rem;
  background: var(--status-failed-bg);
  color: var(--status-failed-text);
  font-size: 0.85rem;
  font-weight: 600;
}

.btn-primary {
  padding: 0.75rem;
  border-radius: 0.5rem;
  background: var(--accent-500);
  color: white;
  font-weight: 700;
  font-size: 1rem;
  border: none;
  cursor: pointer;
  transition: background 0.2s;

  &:hover:not(:disabled) {
    background: var(--accent-600);
  }

  &:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
}

.auth-footer {
  text-align: center;
  margin-top: 1.5rem;
  font-size: 0.9rem;
  color: var(--text-500);

  a {
    color: var(--accent-500);
    text-decoration: none;
    font-weight: 600;

    &:hover {
      text-decoration: underline;
    }
  }
}
</style>
