<script setup lang="ts">
import { ref } from "vue"
import { useQuery } from "@tanstack/vue-query"
import { useRouter } from "vue-router"
import { getEnrollmentTokens } from "@/lib/api"
import { useSurrealClient } from "@/lib/surreal"
import QueryState from "@/components/QueryState.vue"

const router = useRouter()
const client = useSurrealClient()

const { data: tokens, isLoading, isError, error } = useQuery({
  queryKey: ["enrollment-tokens"],
  queryFn: () => getEnrollmentTokens(client),
  select: (data) => Object.freeze(data),
})
</script>

<template>
  <div class="page">
    <div class="section-header-row">
      <div>
        <p v-if="tokens && tokens.length > 0" class="page-subtitle">
          {{ tokens.length }} enrollment {{ tokens.length === 1 ? "token" : "tokens" }}
        </p>
      </div>
      <div class="section-actions">
        <button class="btn-primary" @click="router.push('/tokens/new')">
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" style="margin-right: 4px;">
            <line x1="12" y1="5" x2="12" y2="19" />
            <line x1="5" y1="12" x2="19" y2="12" />
          </svg>
          Generate Token
        </button>
      </div>
    </div>

    <QueryState
      :is-loading="isLoading"
      :is-error="isError"
      :error="error"
      :is-empty="!tokens || tokens.length === 0"
      empty-label="No enrollment tokens"
      empty-subtext="Generate a token to allow a new endpoint to enroll."
    >
      <div class="data-table-card">
        <table class="data-table">
          <thead>
            <tr>
              <th>Status</th>
              <th>Type</th>
              <th>Token Hash</th>
              <th>Expires</th>
              <th>Created</th>
              <th>Used By</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="t in tokens" :key="String(t.id)">
              <td>
                <span v-if="!t.valid" class="status-badge failed">Revoked</span>
                <span v-else-if="t.used_at" class="status-badge completed">Used</span>
                <span v-else-if="t.expires_at && new Date(t.expires_at) < new Date()" class="status-badge timedout">Expired</span>
                <span v-else class="status-badge running">Active</span>
              </td>
              <td>
                <span style="font-size: 0.85rem;">{{ t.single_use ? "Single Use" : "Multi Use" }}</span>
              </td>
              <td>
                <span class="monospace" style="font-size: 0.8rem;">{{ t.token_hash.slice(0, 16) }}...</span>
              </td>
              <td>
                <span style="color: var(--text-500); font-size: 0.85rem;">{{ t.expires_at ? new Date(t.expires_at).toLocaleDateString() : "Never" }}</span>
              </td>
              <td>
                <span style="color: var(--text-500); font-size: 0.85rem;">{{ new Date(t.created_at).toLocaleDateString() }}</span>
              </td>
              <td>
                <span style="color: var(--text-500); font-size: 0.85rem;">{{ t.used_by ? String(t.used_by) : "\u2014" }}</span>
              </td>
            </tr>
          </tbody>
        </table>
      </div>
    </QueryState>
  </div>
</template>
