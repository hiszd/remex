import { ref, shallowRef, computed } from "vue"
import { useSurrealClient } from "@/lib/surreal"
import type { Surreal } from "surrealdb"
import type { User } from "@/lib/model"

/** sessionStorage key for the JWT access token */
const ACCESS_TOKEN_KEY = "remex_access_token"
/** sessionStorage key for the user's email (used for loading user record) */
const AUTH_EMAIL_KEY = "remex_auth_email"
/** sessionStorage key for the server-side refresh token */
const REFRESH_TOKEN_KEY = "remex_refresh_token"

/** SurrealDB access configuration */
const NS = "remex"
const DB = "remex"
const AC = "configurator_access"

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
    const parts = token.split(".")
    if (parts.length !== 3) return null
    const payload = JSON.parse(atob(parts[1]))
    return payload.exp ? new Date(payload.exp * 1000) : null
  } catch {
    return null
  }
}

/**
 * Check if the given access token is expired or within 45 seconds of expiry.
 */
export function isAccessTokenExpired(token: string | null): boolean {
  if (!token) return true
  try {
    const expiry = getTokenExpiry(token)
    if (!expiry) return true
    // Proactively mark expired if within 45 seconds of actual boundary
    return Date.now() >= expiry.getTime() - 45_000
  } catch {
    return true
  }
}

/**
 * Clear all persisted auth state and reset the reactive auth store.
 * Call this on logout, token expiry, or any auth failure.
 */
export function clearAuth(): void {
  sessionStorage.removeItem(ACCESS_TOKEN_KEY)
  sessionStorage.removeItem(AUTH_EMAIL_KEY)
  sessionStorage.removeItem(REFRESH_TOKEN_KEY)
  state.value = { user: null }
}

/**
 * Load the current user record by email into the reactive auth store.
 */
async function loadCurrentUser(client: Surreal, email: string): Promise<void> {
  try {
    const r = await client
      .query(
        "SELECT id, username, email, created_at, updated_at FROM user WHERE email = $email",
        { email }
      )
      .collect()
    const user = (r as unknown[])[0] as User | undefined
    if (user) {
      state.value = { user }
    }
  } catch (e) {
    console.error("[auth] loadCurrentUser failed:", e)
    state.value = { user: null }
  }
}

/**
 * Mint a new server-side refresh token after a successful signin.
 * Stores the token in sessionStorage and returns it.
 */
export async function mintRefreshToken(client: Surreal): Promise<string> {
  const result = await client
    .query(
      `{
        LET $token_str = rand::string(64);
        CREATE refresh_token SET
          user = $auth,
          token = $token_str,
          expires = time::now() + 7d;
        RETURN $token_str;
      }`
    )
    .collect()

  const token = (result as unknown[])[0] as string | undefined
  if (!token) throw new Error("Failed to provision session token context")

  sessionStorage.setItem(REFRESH_TOKEN_KEY, token)
  return token
}

/**
 * Rotate the current refresh token: invalidate the old one and mint a new one.
 * First re-authenticates with the old refresh token, then performs the rotation.
 * Stores the new token in sessionStorage and returns it.
 */
export async function rotateRefreshToken(client: Surreal): Promise<string> {
  const oldToken = sessionStorage.getItem(REFRESH_TOKEN_KEY)
  if (!oldToken) throw new Error("No active refresh token found in storage")

  // Re-authenticate the connection using the token exchange parameter
  await client.signin({
    namespace: NS,
    database: DB,
    access: AC,
    variables: { refresh_token: oldToken },
  })

  const result = await client
    .query(
      `{
        UPDATE refresh_token SET active = false, revoked_at = time::now() WHERE token = $old_token;
        LET $new_token_str = rand::string(64);
        CREATE refresh_token SET
          user = $auth,
          token = $new_token_str,
          expires = time::now() + 7d;
        RETURN $new_token_str;
      }`,
      { old_token: oldToken }
    )
    .collect()

  const newToken = (result as unknown[])[0] as string | undefined
  if (!newToken) throw new Error("Token rotation rejected by database engine")

  sessionStorage.setItem(REFRESH_TOKEN_KEY, newToken)
  return newToken
}

/**
 * Attempt to restore a session from sessionStorage using a stored refresh token.
 * Returns true if the session was restored successfully, false otherwise.
 */
export async function tryRestoreSession(client: Surreal): Promise<boolean> {
  const storedRefreshToken = sessionStorage.getItem(REFRESH_TOKEN_KEY)
  const email = sessionStorage.getItem(AUTH_EMAIL_KEY)
  if (!storedRefreshToken || !email) {
    authReady.value = true
    return false
  }

  try {
    // Re-authenticate the client instance using the saved refresh token
    const tokens = await client.signin({
      namespace: NS,
      database: DB,
      access: AC,
      variables: { refresh_token: storedRefreshToken },
    })

    // Store the new access token
    sessionStorage.setItem(ACCESS_TOKEN_KEY, tokens.access)
    await client.use({ namespace: NS, database: DB })

    // Load the user record
    await loadCurrentUser(client, email)

    // Proactively rotate to reset the server grace window
    await rotateRefreshToken(client)

    authReady.value = true
    return true
  } catch {
    clearAuth()
    authReady.value = true
    return false
  }
}

/**
 * Clear the server-side refresh token and local auth state.
 */
export async function clearAuthSession(client: Surreal): Promise<void> {
  const oldToken = sessionStorage.getItem(REFRESH_TOKEN_KEY)
  if (oldToken) {
    try {
      await client.query(
        "UPDATE refresh_token SET active = false, revoked_at = time::now() WHERE token = $old_token;",
        { old_token: oldToken }
      )
    } catch {
      // Fail silently on server cleanup during hard sign-out
    }
  }
  clearAuth()
  try {
    await client.invalidate()
  } catch {
    // Ignore cleanup errors
  }
}

export function useAuth() {
  const client = useSurrealClient()

  const currentUser = computed(() => state.value.user)
  const isAuthenticated = computed(() => state.value.user !== null)

  async function login(email: string, password: string): Promise<void> {
    // Authenticate with credentials (Mode A)
    const tokens = await client.signin({
      namespace: NS,
      database: DB,
      access: AC,
      variables: { email, pass: password },
    })

    // Store the access token
    sessionStorage.setItem(ACCESS_TOKEN_KEY, tokens.access)
    sessionStorage.setItem(AUTH_EMAIL_KEY, email)

    await client.use({ namespace: NS, database: DB })

    // Mint a server-side refresh token
    await mintRefreshToken(client)

    // Load the user record
    await loadCurrentUser(client, email)
    authReady.value = true
  }

  async function signup(
    username: string,
    email: string,
    password: string
  ): Promise<void> {
    const tokens = await client.signup({
      namespace: NS,
      database: DB,
      access: AC,
      variables: { username, email, password },
    })

    // Store the access token
    sessionStorage.setItem(ACCESS_TOKEN_KEY, tokens.access)
    sessionStorage.setItem(AUTH_EMAIL_KEY, email)

    await client.use({ namespace: NS, database: DB })

    // Mint a server-side refresh token
    await mintRefreshToken(client)

    // Load the user record
    await loadCurrentUser(client, email)
    authReady.value = true
  }

  async function logout(): Promise<void> {
    await clearAuthSession(client)
  }

  return {
    currentUser,
    isAuthenticated,
    login,
    signup,
    logout,
  }
}
