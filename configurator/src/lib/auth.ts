import { ref, computed } from "vue"
import { useSurrealClient } from "@/lib/surreal"
import type { Surreal } from "surrealdb"
import type { User } from "@/lib/model"

const AUTH_TOKEN_KEY = "remex_auth_token"

interface AuthState {
  user: User | null
}

const state = ref<AuthState>({ user: null })
export const isAuthenticatedGlob = computed(() => state.value.user !== null)
export const authReady = ref(false)

async function loadCurrentUser(client: Surreal): Promise<void> {
  try {
    const result = await client.query(
      "SELECT id, username, email, created_at, updated_at FROM user WHERE id = $auth.id"
    )
    const rows = result?.[0]
    if (rows && rows.length > 0) {
      state.value = { user: rows[0] as unknown as User }
    }
  } catch {
    state.value = { user: null }
  }
}

export async function tryRestoreSession(client: Surreal): Promise<boolean> {
  const token = localStorage.getItem(AUTH_TOKEN_KEY)
  if (!token) {
    authReady.value = true
    return false
  }

  try {
    await client.authenticate(token)
    await loadCurrentUser(client)
    authReady.value = true
    return true
  } catch {
    localStorage.removeItem(AUTH_TOKEN_KEY)
    state.value = { user: null }
    authReady.value = true
    return false
  }
}

export function useAuth() {
  const client = useSurrealClient()

  const currentUser = computed(() => state.value.user)
  const isAuthenticated = computed(() => state.value.user !== null)

  async function login(email: string, password: string): Promise<void> {
    const token = await client.signin({
      namespace: "remex",
      database: "remex",
      access: "configurator_access",
      variables: { email, password },
    })
    localStorage.setItem(AUTH_TOKEN_KEY, token)
    await loadCurrentUser(client)
  }

  async function signup(
    username: string,
    email: string,
    password: string
  ): Promise<void> {
    const token = await client.signup({
      namespace: "remex",
      database: "remex",
      access: "configurator_access",
      variables: { username, email, password },
    })
    localStorage.setItem(AUTH_TOKEN_KEY, token)
    await loadCurrentUser(client)
  }

  async function logout(): Promise<void> {
    try {
      await client.invalidate()
    } catch {
      // ignore cleanup errors
    }
    localStorage.removeItem(AUTH_TOKEN_KEY)
    state.value = { user: null }
  }

  return {
    currentUser,
    isAuthenticated,
    login,
    signup,
    logout,
  }
}
