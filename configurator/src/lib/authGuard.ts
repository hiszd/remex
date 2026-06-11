/**
 * ============================================================================
 * authGuard.ts — Global authentication guard for the Remex Configurator
 * ============================================================================
 *
 * This module provides:
 *
 * 1. **Inactivity tracking** — Monitors user activity (keyboard, click, touch).
 *    After 15 minutes of inactivity, the session is revoked server-side and
 *    the user is redirected to `/login`. Event resets are throttled to once
 *    per 60 seconds to prevent rapid timer resets from continuous interaction.
 *
 * 2. **Proactive token refresh** — Before the access token expires, the guard
 *    rotates the refresh token to obtain a new access token.
 *
 * 3. **Reactive error handling** — If a query or mutation fails due to an
 *    expired/invalid token, the guard clears auth state and redirects to login.
 *
 * ## How to reuse on a new page
 *
 * No per-page setup is needed. `useAuthGuard()` is mounted once in `App.vue`
 * and applies globally. Every page automatically benefits from:
 *   - Inactivity timeout (15 minutes)
 *   - Automatic token rotation before access token expiry
 *   - Automatic redirect to `/login` on auth failure
 *
 * After a successful login, call `setupGlobalInactivityListeners(client, router)`
 * to activate the inactivity tracker for that session.
 *
 * ## Token lifecycle
 *
 *   signin/signup → access token (15m) + refresh token (7d, server-side)
 *   └─> inactivity timer: 15min of no interaction → revoke + redirect (throttled to 1/min)
 *   └─> rotation timer: fires when access token is within 45s of expiry
 *       └─> rotateRefreshToken() → new access + refresh tokens
 *           └─> schedule next rotation
 *
 * If rotation fails (refresh token expired or revoked), the user is redirected
 * to `/login` and must re-authenticate.
 */

import { onMounted, onUnmounted } from "vue"
import { useQueryClient } from "@tanstack/vue-query"
import { useRouter, type Router } from "vue-router"
import type { Surreal } from "surrealdb"
import {
  clearAuth,
  clearAuthSession,
  rotateRefreshToken,
  isAccessTokenExpired,
} from "@/lib/auth"

/** Inactivity timeout in milliseconds (15 minutes) */
const INACTIVITY_TIMEOUT_MS = 15 * 60 * 1000

/** sessionStorage key for the access token (must match auth.ts) */
const ACCESS_TOKEN_KEY = "remex_access_token"

// ---------------------------------------------------------------------------
// Inactivity Tracker
// ---------------------------------------------------------------------------

let inactivityTimer: ReturnType<typeof setTimeout> | null = null
let lastResetTime = 0
const RESET_THROTTLE_MS = 60 * 1000 // max once per minute

/**
 * Reset the inactivity timer. Called on every user interaction.
 * Throttled to fire at most once per 60 seconds to prevent rapid resets.
 * If the timer fires, the session is revoked and the user is redirected.
 */
export function resetInactivityTimer(client: Surreal, router: Router): void {
  const now = Date.now()
  if (now - lastResetTime < RESET_THROTTLE_MS) return
  lastResetTime = now

  if (inactivityTimer) clearTimeout(inactivityTimer)
  inactivityTimer = setTimeout(async () => {
    await handleSignOut(client, router)
  }, INACTIVITY_TIMEOUT_MS)
}

/**
 * Revoke the session server-side, clear local state, and redirect to login.
 */
export async function handleSignOut(
  client: Surreal,
  router: Router
): Promise<void> {
  if (inactivityTimer) clearTimeout(inactivityTimer)
  await clearAuthSession(client)
  router.push("/login")
}

/**
 * Register global event listeners for inactivity tracking.
 * Call this once after a successful login or session restore.
 */
export function setupGlobalInactivityListeners(
  client: Surreal,
  router: Router
): void {
  const events = ["keydown", "click", "touchstart"]
  const reset = () => resetInactivityTimer(client, router)

  events.forEach((event) => {
    window.addEventListener(event, reset, { passive: true })
  })

  // Initial activation
  reset()
}

/**
 * Remove global inactivity event listeners.
 * Call this on logout or when the app is unmounting.
 */
export function teardownGlobalInactivityListeners(): void {
  if (inactivityTimer) {
    clearTimeout(inactivityTimer)
    inactivityTimer = null
  }
  const events = ["keydown", "click", "touchstart"]
  events.forEach((event) => {
    window.removeEventListener(event, resetInactivityTimer as EventListener)
  })
}

// ---------------------------------------------------------------------------
// Token Rotation Timer
// ---------------------------------------------------------------------------

/**
 * Sets up a timer that fires when the access token is close to expiry.
 * On fire, calls `rotateRefreshToken()` to obtain a new access token.
 * If rotation succeeds, schedules the next rotation cycle.
 * If rotation fails, clears auth and redirects to `/login`.
 */
function useTokenRotationTimer(client: Surreal) {
  const router = useRouter()
  let timer: ReturnType<typeof setTimeout> | null = null

  /**
   * Calculate the delay until the access token is 45 seconds from expiry,
   * then set a timeout to trigger rotation.
   * If the token is already within the buffer window, rotate immediately.
   */
  function scheduleRotation(): void {
    if (timer) clearTimeout(timer)

    const token = sessionStorage.getItem(ACCESS_TOKEN_KEY)
    if (!token) return

    const expiry = getTokenExpiryInternal(token)
    if (!expiry) return

    const rotateAt = expiry.getTime() - 45_000 // 45 seconds before expiry
    const delay = rotateAt - Date.now()

    if (delay <= 0) {
      attemptRotation()
      return
    }

    timer = setTimeout(() => {
      attemptRotation()
    }, delay)
  }

  async function attemptRotation(): Promise<void> {
    try {
      await rotateRefreshToken(client)
      scheduleRotation() // schedule next rotation cycle
    } catch {
      // Rotation failed — clear auth and redirect
      clearAuth()
      router.push("/login")
    }
  }

  onMounted(() => {
    scheduleRotation()
  })

  onUnmounted(() => {
    if (timer) clearTimeout(timer)
  })

  return { scheduleRotation }
}

/**
 * Decode the `exp` claim from a JWT. Internal helper for the rotation timer.
 */
function getTokenExpiryInternal(token: string): Date | null {
  try {
    const parts = token.split(".")
    if (parts.length !== 3) return null
    const payload = JSON.parse(atob(parts[1]))
    return payload.exp ? new Date(payload.exp * 1000) : null
  } catch {
    return null
  }
}

// ---------------------------------------------------------------------------
// Auth Error Handler (Vue Query cache subscription)
// ---------------------------------------------------------------------------

/**
 * Checks whether an error object indicates an authentication failure.
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
 * token expiry and the rotation timer.
 */
function useAuthErrorHandler() {
  const queryClient = useQueryClient()
  const router = useRouter()

  onMounted(() => {
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
 *   - Token rotation timer before access token expiry
 *   - Reactive redirect on auth errors from any query/mutation
 *
 * Inactivity tracking is activated separately via `setupGlobalInactivityListeners()`.
 *
 * Returns `{ scheduleRotation }` so the caller can trigger a rotation cycle
 * after restoring a session from sessionStorage.
 */
export function useAuthGuard(client: Surreal) {
  const { scheduleRotation } = useTokenRotationTimer(client)
  useAuthErrorHandler()
  return { scheduleRotation }
}
