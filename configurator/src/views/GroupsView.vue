<script setup lang="ts">
import { getGroups } from "@/lib/api"
import { useQuery } from "@tanstack/vue-query"
import { useSurrealClient } from "@/lib/surreal"
import GroupComponent from "@/components/Group.vue"
import QueryState from "@/components/QueryState.vue"

const client = useSurrealClient()

const { data: groups, isLoading, isError, error } = useQuery({
  queryKey: ["groups"],
  queryFn: () => getGroups(client),
  select: (data) => Object.freeze(data),
})
</script>

<template>
  <div class="page">
    <header class="page-header">
      <div class="header-top">
        <h1 class="page-title">Groups</h1>
        <router-link to="/groups/new" class="btn-primary">Create Group</router-link>
      </div>
      <p class="page-subtitle" v-if="groups && groups.length > 0">
        {{ groups.length }} {{ groups.length === 1 ? "group" : "groups" }}
      </p>
    </header>

    <QueryState
      :is-loading="isLoading"
      :is-error="isError"
      :error="error"
      :is-empty="!groups || groups.length === 0"
      empty-label="No groups yet"
      empty-subtext="Groups will appear here once they are created."
    >
      <div class="card-grid">
        <GroupComponent
          v-for="group in groups"
          :group="group"
          :key="String(group.id)"
        />
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
