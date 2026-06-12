import { ref, shallowRef, computed } from "vue"
import { useSurrealClient } from "@/lib/surreal"
import { RecordId, type Surreal } from "surrealdb"
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
 * Extract the authenticated user's record ID from the JWT access token.
 * SurrealDB record access tokens include an `ID` claim with the record ID
 * (e.g., "user:xyz"). Returns a RecordId object for use in queries.
 */
function getUserIdFromToken(token: string): RecordId | null {
  try {
    const parts = token.split(".")
    if (parts.length !== 3) return null
    const payload = JSON.parse(atob(parts[1]))
    const id = payload.ID
    if (!id || typeof id !== "string") return null
    const sep = id.indexOf(":")
    if (sep === -1) return null
    return new RecordId(id.slice(0, sep), id.slice(sep + 1))
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

    // collect() returns [results]. For SELECT, results is an array of records.
    const records = Array.isArray(r) ? (r[0] as unknown[]) : []
    const user = records[0] as User | undefined
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
export async function mintRefreshToken(client: Surreal, accessToken: string): Promise<string> {
  const userId = getUserIdFromToken(accessToken)
  if (!userId) {
    throw new Error("Failed to extract user ID from access token")
  }

  const tokenStr = Array.from(crypto.getRandomValues(new Uint8Array(48)))
    .map(b => b.toString(36))
    .join("")
    .slice(0, 64)

  const createResult = await client
    .query(
      "CREATE refresh_token SET user = $user_id, token = $token_val, expires = time::now() + 7d",
      { user_id: userId, token_val: tokenStr }
    )
    .collect()

  if (!createResult || (Array.isArray(createResult) && createResult.length === 0)) {
    throw new Error("Failed to create refresh_token record")
  }

  sessionStorage.setItem(REFRESH_TOKEN_KEY, tokenStr)
  return tokenStr
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
  const tokens = await client.signin({
    namespace: NS,
    database: DB,
    access: AC,
    variables: { refresh_token: oldToken },
  })

  // Extract user ID from the new access token
  const userId = getUserIdFromToken(tokens.access)
  if (!userId) {
    throw new Error("Failed to extract user ID from rotated access token")
  }

  // Invalidate old token
  await client.query(
    "UPDATE refresh_token SET active = false, revoked_at = time::now() WHERE token = $old_token",
    { old_token: oldToken }
  )

  // Generate new token and create record
  const tokenStr = Array.from(crypto.getRandomValues(new Uint8Array(48)))
    .map(b => b.toString(36))
    .join("")
    .slice(0, 64)

  const createResult = await client
    .query(
      "CREATE refresh_token SET user = $user_id, token = $token_val, expires = time::now() + 7d",
      { user_id: userId, token_val: tokenStr }
    )
    .collect()

  if (!createResult || (Array.isArray(createResult) && createResult.length === 0)) {
    throw new Error("Token rotation rejected by database engine")
  }

  sessionStorage.setItem(REFRESH_TOKEN_KEY, tokenStr)
  return tokenStr
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
    await mintRefreshToken(client, tokens.access)

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
    await mintRefreshToken(client, tokens.access)

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
