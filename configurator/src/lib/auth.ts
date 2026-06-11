import { ref, shallowRef, computed } from "vue"
import { useSurrealClient } from "@/lib/surreal"
import type { Surreal } from "surrealdb"
import type { User } from "@/lib/model"

/** localStorage key for the JWT access token */
const AUTH_TOKEN_KEY = "remex_auth_token"
/** localStorage key for the user's email (used for session restore) */
const AUTH_EMAIL_KEY = "remex_auth_email"
/** localStorage key for the JWT refresh token */
const AUTH_REFRESH_KEY = "remex_refresh_token"

interface AuthState {
  user: User | null
}

const state = shallowRef<AuthState>({ user: null })
export const isAuthenticatedGlob = computed(() => state.value.user !== null)
export const authReady = ref(false)

/**
 * Decode the `exp` claim from a JWT to get its absolute expiry time.
 * Returns null if the token cannot be parsed or has no `exp` claim.
 */
export function getTokenExpiry(token: string): Date | null {
  try {
    const parts = token.split('.')
    if (parts.length !== 3) return null
    const payload = JSON.parse(atob(parts[1]))
    return payload.exp ? new Date(payload.exp * 1000) : null
  } catch {
    return null
  }
}

/**
 * Clear all persisted auth state and reset the reactive auth store.
 * Call this on logout, token expiry, or any auth failure.
 */
export function clearAuth(): void {
  localStorage.removeItem(AUTH_TOKEN_KEY)
  localStorage.removeItem(AUTH_EMAIL_KEY)
  localStorage.removeItem(AUTH_REFRESH_KEY)
  state.value = { user: null }
}

async function loadCurrentUser(client: Surreal, email: string): Promise<void> {
  try {
    const r = await client.query(
      "SELECT id, username, email, created_at, updated_at FROM user WHERE email = $email",
      { email }
    ).collect()
    const user = (r?.[0] as unknown) as User | undefined
    console.log("[auth] loadCurrentUser query returned:", user)
    if (user) {
      state.value = { user }
    }
  } catch (e) {
    console.error("[auth] loadCurrentUser failed:", e)
    state.value = { user: null }
  }
}

export async function tryRestoreSession(client: Surreal): Promise<boolean> {
  const token = localStorage.getItem(AUTH_TOKEN_KEY)
  const email = localStorage.getItem(AUTH_EMAIL_KEY)
  if (!token || !email) {
    authReady.value = true
    return false
  }

  try {
    await client.authenticate(token)
    await client.use({ namespace: "remex", database: "remex" })
    await loadCurrentUser(client, email)
    authReady.value = true
    return true
  } catch {
    // Access token is expired — try to refresh if we have a valid refresh token
    const refreshToken = localStorage.getItem(AUTH_REFRESH_KEY)
    if (refreshToken) {
      try {
        const newTokens = await client.refresh({
          access: "",
          refresh: refreshToken,
        })
        localStorage.setItem(AUTH_TOKEN_KEY, newTokens.access)
        if (newTokens.refresh) {
          localStorage.setItem(AUTH_REFRESH_KEY, newTokens.refresh)
        }
        await client.authenticate(newTokens.access)
        await client.use({ namespace: "remex", database: "remex" })
        await loadCurrentUser(client, email)
        authReady.value = true
        return true
      } catch {
        // Refresh token also expired — full logout
        clearAuth()
        authReady.value = true
        return false
      }
    }

    clearAuth()
    authReady.value = true
    return false
  }
}

export function useAuth() {
  const client = useSurrealClient()

  const currentUser = computed(() => state.value.user)
  const isAuthenticated = computed(() => state.value.user !== null)

  async function login(email: string, password: string): Promise<void> {
    console.log("[auth] login() start")
    const token = await client.signin({
      namespace: "remex",
      database: "remex",
      access: "configurator_access",
      variables: { email, password },
    })
    console.log("[auth] signin success")
    localStorage.setItem(AUTH_TOKEN_KEY, token.access)
    if (token.refresh) {
      localStorage.setItem(AUTH_REFRESH_KEY, token.refresh)
    }
    localStorage.setItem(AUTH_EMAIL_KEY, email)
    await client.authenticate(token.access)
    await client.use({ namespace: "remex", database: "remex" })
    await loadCurrentUser(client, email)
    console.log("[auth] login() complete, user:", state.value.user)
    authReady.value = true
  }

  async function signup(
    username: string,
    email: string,
    password: string
  ): Promise<void> {
    console.log("[auth] signup() start")
    const token = await client.signup({
      namespace: "remex",
      database: "remex",
      access: "configurator_access",
      variables: { username, email, password },
    })
    console.log("[auth] signup success")
    localStorage.setItem(AUTH_TOKEN_KEY, token.access)
    if (token.refresh) {
      localStorage.setItem(AUTH_REFRESH_KEY, token.refresh)
    }
    localStorage.setItem(AUTH_EMAIL_KEY, email)
    await client.authenticate(token.access)
    await client.use({ namespace: "remex", database: "remex" })
    await loadCurrentUser(client, email)
    console.log("[auth] signup() complete, user:", state.value.user)
    authReady.value = true
  }

  async function logout(): Promise<void> {
    try {
      await client.invalidate()
    } catch {
      // ignore cleanup errors
    }
    clearAuth()
  }

  return {
    currentUser,
    isAuthenticated,
    login,
    signup,
    logout,
  }
}
