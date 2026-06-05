<script setup lang="ts">
import { getJobs } from "@/lib/api"
import { useQuery } from "@tanstack/vue-query"
import { useSurrealClient } from "@/lib/surreal"
import JobComponent from "@/components/Job.vue"

const client = useSurrealClient()

const { data: jobs, isLoading, isError, error } = useQuery({
  queryKey: ["jobs"],
  queryFn: () => getJobs(client),
})
</script>

<template>
  <div class="page">
    <header class="page-header">
      <div class="header-top">
        <h1 class="page-title">Jobs</h1>
        <router-link to="/jobs/new" class="btn-primary">Create Job</router-link>
      </div>
      <p class="page-subtitle" v-if="jobs && jobs.length > 0">
        {{ jobs.length }} configured {{ jobs.length === 1 ? "job" : "jobs" }}
      </p>
    </header>

    <div v-if="isLoading" class="state-card">
      <div class="spinner" />
      <p class="state-text">Loading jobs...</p>
    </div>

    <div v-else-if="isError" class="state-card state-error">
      <p class="state-label">Something went wrong</p>
      <p class="state-text">{{ error }}</p>
    </div>

    <div v-else-if="!jobs || jobs.length === 0" class="state-card state-empty">
      <p class="state-label">No jobs found</p>
      <p class="state-text">Jobs will appear here once they are created.</p>
    </div>

    <div v-else class="card-grid">
      <div v-for="job in jobs" :key="String(job.id)">
        <JobComponent :job="job" />
      </div>
    </div>
  </div>
</template>

<style lang="scss" scoped>
.page {
  width: 100%;
  max-width: 1400px;
  margin: 0 auto;
}

.page-header {
  .header-top {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }
}

.card-grid {
  display: flex;
  flex-direction: column;
  gap: 1.5rem;
  width: 100%;
}
</style>
