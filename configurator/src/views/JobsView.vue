<script setup lang="ts">
import { getJobs } from "@/lib/api"
import { useQuery } from "@tanstack/vue-query"
import { useSurrealClient } from "@/lib/surreal"
import JobComponent from "@/components/Job.vue"
import QueryState from "@/components/QueryState.vue"

const client = useSurrealClient()

const { data: jobs, isLoading, isError, error } = useQuery({
  queryKey: ["jobs"],
  queryFn: () => getJobs(client),
  select: (data) => Object.freeze(data),
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

    <QueryState
      :is-loading="isLoading"
      :is-error="isError"
      :error="error"
      :is-empty="!jobs || jobs.length === 0"
      empty-label="No jobs found"
      empty-subtext="Jobs will appear here once they are created."
    >
      <div class="card-grid">
        <div v-for="job in jobs" :key="String(job.id)">
          <JobComponent :job="job" />
        </div>
      </div>
    </QueryState>
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
