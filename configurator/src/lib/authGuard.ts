/**
 * ============================================================================
 * authGuard.ts — Global authentication guard for the Remex Configurator
 * ============================================================================
 *
 * This module provides `useAuthGuard()`, a composable that handles:
 *
 * 1. **Proactive token refresh** — Before the access token expires, the guard
 *    uses the stored refresh token to obtain a new access token via
 *    `client.refresh()`. The user never sees an interruption.
 *
 * 2. **Reactive error handling** — If a query or mutation fails due to an
 *    expired/invalid token (e.g., race condition before refresh fires), the
 *    guard clears auth state and redirects to the login page.
 *
 * ## How to reuse on a new page
 *
 * No per-page setup is needed. `useAuthGuard()` is mounted once in `App.vue`
 * and applies globally. Every page automatically benefits from:
 *   - Automatic token refresh before expiry
 *   - Automatic redirect to `/login` on auth failure
 *
 * If you create a new page, just ensure:
 *   - The router guard in `router/index.ts` checks `isAuthenticatedGlob`
 *     (already configured)
 *   - Your page uses `useQuery`/`useMutation` from `@tanstack/vue-query`
 *     (the error handler intercepts auth errors from these)
 *
 * ## Token lifecycle
 *
 *   signin/signup → access token (1h) + refresh token (7d)
 *   └─> refresh timer fires 5min before access token expiry
 *       └─> client.refresh({ refresh }) → new access + refresh tokens
 *           └─> schedule next refresh
 *
 * If refresh fails (refresh token also expired), the user is redirected to
 * `/login` and must re-authenticate.
 */

import { onMounted, onUnmounted } from "vue"
import { useQueryClient } from "@tanstack/vue-query"
import { useRouter } from "vue-router"
import type { Surreal } from "surrealdb"
import { clearAuth, getTokenExpiry } from "@/lib/auth"

/** How many milliseconds before access token expiry to trigger a refresh */
const REFRESH_BUFFER_MS = 59 * 60 * 1000 // 5 minutes

/** localStorage key for the refresh token (must match auth.ts) */
const AUTH_REFRESH_KEY = "remex_refresh_token"
const AUTH_TOKEN_KEY = "remex_auth_token"

// ---------------------------------------------------------------------------
// Token Refresh Timer
// ---------------------------------------------------------------------------

/**
 * Sets up a timer that fires shortly before the access token expires.
 * On fire, calls `client.refresh()` with the stored refresh token to obtain
 * a new access token. If refresh succeeds, schedules the next refresh.
 * If refresh fails, clears auth and redirects to `/login`.
 */
function useTokenRefreshTimer(client: Surreal) {
  const router = useRouter()
  let timer: ReturnType<typeof setTimeout> | null = null

  /**
   * Attempt to refresh the access token using the stored refresh token.
   * On success: stores new tokens and schedules the next refresh cycle.
   * On failure: clears all auth state and redirects to login.
   */
  async function attemptRefresh(): Promise<void> {
    const refreshToken = localStorage.getItem(AUTH_REFRESH_KEY)
    if (!refreshToken) {
      clearAuth()
      router.push("/login")
      return
    }

    try {
      // Exchange the refresh token for a new access + refresh token pair
      const newTokens = await client.refresh({
        access: "", // not used by the engine, but required by the type
        refresh: refreshToken,
      })

      // Persist the new tokens
      localStorage.setItem(AUTH_TOKEN_KEY, newTokens.access)
      if (newTokens.refresh) {
        localStorage.setItem(AUTH_REFRESH_KEY, newTokens.refresh)
      }

      // Re-authenticate the session with the new access token
      await client.authenticate(newTokens.access)
      await client.use({ namespace: "remex", database: "remex" })
      console.log("[auth] refreshed token")

      // Schedule the next refresh cycle
      scheduleRefresh()
    } catch {
      // Refresh token is invalid or expired — force re-login
      clearAuth()
      router.push("/login")
    }
  }

  /**
   * Calculate the delay until the access token is 5 minutes from expiry,
   * then set a timeout to trigger `attemptRefresh()`.
   * If the token is already within the buffer window, refresh immediately.
   */
  function scheduleRefresh(): void {
    const token = localStorage.getItem(AUTH_TOKEN_KEY)
    if (!token) return

    const expiry = getTokenExpiry(token)
    if (!expiry) return

    const refreshAt = expiry.getTime() - REFRESH_BUFFER_MS
    const delay = refreshAt - Date.now()
    console.log("[auth] scheduling token refresh in", displayDelay(delay))

    if (delay <= 0) {
      // Token is already within the refresh buffer (or expired)
      attemptRefresh()
      return
    }

    timer = setTimeout(() => {
      console.log("refreshing token")
      attemptRefresh()
    }, delay)
  }

  onMounted(() => {
    scheduleRefresh()
  })

  onUnmounted(() => {
    if (timer) clearTimeout(timer)
  })

  // Expose scheduleRefresh so App.vue can call it after session restore
  return { scheduleRefresh }
}

function displayDelay(delay: number): string {
  // if the delay (which is in milliseconds) is longer than 60 seconds, show minutes and seconds, if the delay is longer that
  // 60 minues, show hours, minutes and seconds, if it less than 60 seconds, show seconds
  if (delay > 60 * 60 * 1000) {
    const hours = Math.floor(delay / (60 * 60 * 1000))
    const minutes = Math.floor((delay % (60 * 60 * 1000)) / (60 * 1000))
    const seconds = Math.floor((delay % (60 * 1000)) / 1000)
    return `${hours} hours ${minutes} minutes ${seconds} seconds`
  } else if (delay > 60 * 1000) {
    const minutes = Math.floor(delay / (60 * 1000))
    const seconds = Math.floor((delay % (60 * 1000)) / 1000)
    return `${minutes} minutes ${seconds} seconds`
  } else {
    const seconds = Math.floor(delay / 1000)
    return `${seconds} seconds`
  }
}

// ---------------------------------------------------------------------------
// Auth Error Handler (Vue Query cache subscription)
// ---------------------------------------------------------------------------

/**
 * Checks whether an error object indicates an authentication failure.
 * SurrealDB returns messages like "There was a problem with the authentication",
 * "Token has expired", "Unauthorized", etc.
 */
function isAuthError(error: unknown): boolean {
  if (error instanceof Error) {
    const msg = error.message.toLowerCase()
    return (
      msg.includes("authentication") ||
      msg.includes("token") ||
      msg.includes("unauthorized") ||
      msg.includes("forbidden")
    )
  }
  return false
}

/**
 * Subscribes to Vue Query's query and mutation caches. When any query or
 * mutation fails with an auth-related error, clears auth state and redirects
 * to the login page.
 *
 * This acts as a safety net for race conditions where a query fires between
 * token expiry and the refresh timer.
 */
function useAuthErrorHandler() {
  const queryClient = useQueryClient()
  const router = useRouter()

  onMounted(() => {
    // Watch the query cache for auth errors
    const unsubQuery = queryClient.getQueryCache().subscribe((event) => {
      if (
        event.type === "updated" &&
        event.action === "error" &&
        isAuthError(event.query.state.error)
      ) {
        clearAuth()
        router.push("/login")
      }
    })

    // Watch the mutation cache for auth errors
    const unsubMutation = queryClient.getMutationCache().subscribe((event) => {
      if (
        event.type === "updated" &&
        event.action === "error" &&
        isAuthError(event.mutation.state.error)
      ) {
        clearAuth()
        router.push("/login")
      }
    })

    onUnmounted(() => {
      unsubQuery()
      unsubMutation()
    })
  })
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/**
 * Mount the global authentication guard. Call this once at the app root
 * (App.vue). It activates:
 *   - Proactive token refresh before access token expiry
 *   - Reactive redirect on auth errors from any query/mutation
 *
 * Returns `{ scheduleRefresh }` so the caller can trigger a refresh cycle
 * after restoring a session from localStorage.
 */
export function useAuthGuard(client: Surreal) {
  const { scheduleRefresh } = useTokenRefreshTimer(client)
  useAuthErrorHandler()
  return { scheduleRefresh }
}
