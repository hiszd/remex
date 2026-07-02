<script setup lang="ts">
import { computed } from "vue"
import { useRoute, useRouter } from "vue-router"
import { useQuery } from "@tanstack/vue-query"
import { getEnrollmentTokenById, getClients, getUserById } from "@/lib/api"
import { useSurrealClient } from "@/lib/surreal"
import { formatDate } from "@/utils/date"
import type { RecordId } from "surrealdb"

const route = useRoute()
const router = useRouter()
const surreal = useSurrealClient()
const tokenId = route.params.id as string

const { data: token, isLoading, isError, error } = useQuery({
  queryKey: ["enrollment-token", tokenId],
  queryFn: () => getEnrollmentTokenById(surreal, tokenId),
  select: (data) => Object.freeze(data),
  retry: 0,
})

const { data: allClients } = useQuery({
  queryKey: ["clients"],
  queryFn: () => getClients(surreal),
  select: (data) => Object.freeze(data),
})

const tokenStatus = computed(() => {
  if (!token.value) return ""
  if (!token.value.valid) return "Revoked"
  if (token.value.last_used && token.value.single_use) return "Used"
  if (token.value.expires_at && new Date(token.value.expires_at) < new Date()) return "Expired"
  return "Active"
})

const tokenStatusClass = computed(() => {
  const s = tokenStatus.value
  if (s === "Active") return "running"
  if (s === "Used") return "completed"
  if (s === "Expired") return "timedout"
  return "failed"
})

const tokenType = computed(() =>
  token.value?.single_use ? "Single Use" : "Multi Use"
)

const { data: issuedByUser } = useQuery({
  queryKey: ["user", token.value?.issued_by],
  queryFn: () => {
    if (!token.value?.issued_by) return null
    return getUserById(surreal, String(token.value.issued_by))
  },
  enabled: () => !!token.value?.issued_by,
  retry: 0,
})

function clientName(id: string): string {
  if (!allClients.value) return id
  const c = allClients.value.find((c) => String(c.id) === id)
  return c?.client_name ?? id
}

function displayId(id: RecordId): string {
  return `${id.table}:${id.id}`
}
</script>

<template>
  <div class="page">
    <router-link to="/tokens" class="back-link">← Back to Tokens</router-link>

    <div v-if="token" class="header-main">
      <div class="title-group">
        <h1 class="page-title">Token Details</h1>
        <p class="record-id">{{ displayId(token.id) }}</p>
      </div>
    </div>

    <div v-if="isLoading" class="state-card">
      <div class="spinner" />
      <p class="state-text">Loading token details...</p>
    </div>

    <div v-else-if="isError || !token" class="state-card state-error">
      <p class="state-label">Token not found</p>
      <p class="state-text">The requested enrollment token could not be retrieved.</p>
      <p v-if="error" class="state-text" style="color:var(--danger);margin-top:0.5rem;font-size:0.8rem">{{ error.message }}</p>
    </div>

    <template v-else>
      <section class="card">
        <div class="section-header">
          <h2>General Information</h2>
        </div>

        <div class="info-grid">
          <div class="info-item">
            <span class="label">Status</span>
            <span class="value status-badge" :class="tokenStatusClass">{{ tokenStatus }}</span>
          </div>
          <div class="info-item">
            <span class="label">Type</span>
            <span class="value">{{ tokenType }}</span>
          </div>
          <div class="info-item">
            <span class="label">Token Hash</span>
            <span class="value monospace" style="font-size:0.8rem;word-break:break-all;">{{ token.token_hash }}</span>
          </div>
          <div class="info-item">
            <span class="label">Valid</span>
            <span class="value">{{ token.valid ? "Yes" : "No" }}</span>
          </div>
          <div class="info-item">
            <span class="label">Expires</span>
            <span class="value">{{ token.expires_at ? formatDate(token.expires_at) : "Never" }}</span>
          </div>
          <div class="info-item">
            <span class="label">Created</span>
            <span class="value">{{ formatDate(token.created_at) }}</span>
          </div>
          <div class="info-item">
            <span class="label">Issued By</span>
            <span class="value">{{ issuedByUser?.username ?? String(token.issued_by) }}</span>
          </div>
          <div class="info-item">
            <span class="label">Last Used</span>
            <span class="value">{{ token.last_used ? formatDate(token.last_used) : "\u2014" }}</span>
          </div>
          <div class="info-item">
            <span class="label">Last Used By</span>
            <span class="value">
              <template v-if="token.last_used_by">
                <router-link
                  v-if="token.last_used_by"
                  :to="`/clients/${token.last_used_by}`"
                  class="link"
                >{{ clientName(String(token.last_used_by)) }}</router-link>
              </template>
              <template v-else>&mdash;</template>
            </span>
          </div>
        </div>
      </section>

      <section class="card">
        <div class="section-header">
          <h2>Usage History</h2>
        </div>

        <div v-if="!token.usage_history || token.usage_history.length === 0" class="empty-state">
          <p>No usage recorded</p>
        </div>

        <div v-else class="data-table-card">
          <table class="data-table">
            <thead>
              <tr>
                <th>Client</th>
                <th>Used At</th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="(entry, i) in token.usage_history" :key="i">
                <td>
                  <router-link
                    :to="`/clients/${entry.client_id}`"
                    class="link"
                  >{{ clientName(String(entry.client_id)) }}</router-link>
                </td>
                <td>
                  <span style="color: var(--text-500); font-size: 0.85rem;">{{ formatDate(entry.used_at) }}</span>
                </td>
              </tr>
            </tbody>
          </table>
        </div>
      </section>
    </template>
  </div>
</template>

<style lang="scss" scoped>
.link {
  color: var(--accent);
  text-decoration: none;
  font-weight: 600;

  &:hover {
    text-decoration: underline;
  }
}
</style>
