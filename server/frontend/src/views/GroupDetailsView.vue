<script setup lang="ts">
import { useRoute } from 'vue-router';
import { useQuery } from '@tanstack/vue-query';
import { getGroupById, getGroupClients, getGroupJobs, type Group, type ClientSrv, type JobSrv } from '@/client/index';
import { formatDate } from '@/utils/date';

const route = useRoute();
const groupId = route.params.id as string;

const { data: group, isLoading: loadingGroup, isError: groupError } = useQuery({
  queryKey: ['group', groupId],
  queryFn: async () => {
    const { data, error } = await getGroupById({ path: { group_id: groupId } });
    if (error) throw error;
    return data as Group;
  }
});

const { data: clients, isLoading: loadingClients } = useQuery({
  queryKey: ['group-clients', groupId],
  queryFn: async () => {
    const { data, error } = await getGroupClients({ path: { group_id: groupId } });
    if (error) return [];
    return (data as ClientSrv[]) || [];
  }
});

const { data: jobs, isLoading: loadingJobs } = useQuery({
  queryKey: ['group-jobs', groupId],
  queryFn: async () => {
    const { data, error } = await getGroupJobs({ path: { group_id: groupId } });
    if (error) return [];
    return (data as JobSrv[]) || [];
  }
});
</script>

<template>
  <div class="page">
    <header class="page-header">
      <router-link to="/groups" class="back-link">← Back to Groups</router-link>
      <div v-if="group" class="header-main">
        <div class="title-group">
          <h1 class="page-title">{{ group.group_name }}</h1>
          <p class="group-id">{{ group.id }}</p>
        </div>
      </div>
    </header>

    <div v-if="loadingGroup" class="state-card">
      <div class="spinner" />
      <p>Loading group details...</p>
    </div>

    <div v-else-if="groupError || !group" class="state-card state-error">
      <p class="state-label">Group not found</p>
      <p class="state-text">The requested group could not be retrieved.</p>
    </div>

    <div v-else class="details-container">
      <section class="info-section card">
        <div class="section-header">
          <h2>General Information</h2>
        </div>

        <div class="info-grid">
          <div class="info-item">
            <span class="label">Group Name</span>
            <span class="value">{{ group.group_name }}</span>
          </div>
          <div class="info-item">
            <span class="label">ID</span>
            <span class="value monospace">{{ group.id }}</span>
          </div>
          <div class="info-item">
            <span class="label">Created At</span>
            <span class="value">{{ formatDate(group.created_at) }}</span>
          </div>
          <div class="info-item">
            <span class="label">Updated At</span>
            <span class="value">{{ formatDate(group.updated_at) }}</span>
          </div>
        </div>
      </section>

      <section class="clients-section card">
        <div class="section-header">
          <h2>Clients</h2>
          <span v-if="clients && clients.length > 0" class="count-badge">{{ clients.length }}</span>
        </div>

        <div v-if="loadingClients" class="loading-state">
          <div class="spinner-small" />
          <span>Loading clients...</span>
        </div>

        <div v-else-if="!clients || clients.length === 0" class="empty-text">
          No clients assigned to this group.
        </div>

        <div v-else class="entity-list">
          <router-link
            v-for="client in clients"
            :key="client.id"
            :to="'/clients/' + client.id"
            class="entity-chip"
          >
            <span class="entity-name">{{ client.client_name }}</span>
            <span class="entity-id">{{ client.id }}</span>
          </router-link>
        </div>
      </section>

      <section class="jobs-section card">
        <div class="section-header">
          <h2>Jobs</h2>
          <span v-if="jobs && jobs.length > 0" class="count-badge">{{ jobs.length }}</span>
        </div>

        <div v-if="loadingJobs" class="loading-state">
          <div class="spinner-small" />
          <span>Loading jobs...</span>
        </div>

        <div v-else-if="!jobs || jobs.length === 0" class="empty-text">
          No jobs assigned to this group.
        </div>

        <div v-else class="entity-list">
          <router-link
            v-for="job in jobs"
            :key="job.id"
            :to="'/jobs/' + job.id"
            class="entity-chip"
          >
            <span class="entity-name">{{ job.job_name }}</span>
            <span class="entity-id">{{ job.id }}</span>
          </router-link>
        </div>
      </section>
    </div>
  </div>
</template>

<style lang="scss" scoped>
.page {
  display: flex;
  flex-direction: column;
  width: 100%;
  max-width: 1000px;
  margin: 0 auto;
  padding: 2rem 1.5rem;
  gap: 2rem;
}

.back-link {
  display: inline-block;
  margin-bottom: 1rem;
  font-size: 0.9rem;
  color: var(--accent-500);
  text-decoration: none;

  &:hover {
    text-decoration: underline;
  }
}

.header-main {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  gap: 1rem;
}

.title-group {
  .page-title {
    margin: 0;
    font-size: 2rem;
    font-weight: 800;
  }

  .group-id {
    margin: 0.25rem 0 0;
    font-size: 0.9rem;
    font-family: monospace;
    opacity: 0.5;
  }
}

.details-container {
  display: flex;
  flex-direction: column;
  gap: 1.5rem;
}

.card {
  background: var(--background-300);
  color: var(--text-950);
  border-radius: 1rem;
  padding: 1.5rem;
  box-shadow: 0 4px 6px -1px rgba(0, 0, 0, 0.1);
}

.section-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 1.5rem;
  border-bottom: 1px solid rgba(0, 0, 0, 0.1);
  padding-bottom: 0.75rem;

  h2 {
    margin: 0;
    font-size: 1.25rem;
    font-weight: 700;
  }
}

.count-badge {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  min-width: 1.5rem;
  height: 1.5rem;
  padding: 0 0.5rem;
  background: var(--background-200);
  border-radius: 9999px;
  font-size: 0.8rem;
  font-weight: 600;
  color: var(--text-700);
}

.info-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
  gap: 1.5rem;
}

.info-item {
  display: flex;
  flex-direction: column;
  gap: 0.25rem;

  .label {
    font-size: 0.7rem;
    font-weight: 700;
    text-transform: uppercase;
    opacity: 0.6;
  }

  .value {
    font-size: 1rem;

    &.monospace {
      font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, "Liberation Mono", "Courier New", monospace;
      font-size: 0.85rem;
      background: rgba(0, 0, 0, 0.05);
      padding: 0.2rem 0.4rem;
      border-radius: 0.25rem;
      word-break: break-all;
    }
  }
}

.entity-list {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}

.entity-chip {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 0.75rem 1rem;
  background: var(--background-200);
  border: 1px solid rgba(0, 0, 0, 0.1);
  border-radius: 0.5rem;
  text-decoration: none;
  color: inherit;
  transition: background 0.2s, border-color 0.2s;

  &:hover {
    background: var(--background-100);
    border-color: var(--accent-400);
  }

  .entity-name {
    font-weight: 600;
    color: var(--text-900);
  }

  .entity-id {
    font-size: 0.8rem;
    font-family: monospace;
    opacity: 0.5;
  }
}

.loading-state {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  padding: 1rem;
  color: var(--text-500);
  font-size: 0.9rem;
}

.empty-text {
  color: var(--text-500);
  font-size: 0.9rem;
  font-style: italic;
}

.state-card {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 0.75rem;
  padding: 3rem 1.5rem;
  border-radius: 1rem;
  background-color: var(--background-300);
  color: var(--text-950);
  text-align: center;
}

.state-label {
  margin: 0;
  font-size: 1.1rem;
  font-weight: 700;
}

.state-text {
  margin: 0;
  font-size: 0.9rem;
  opacity: 0.7;
}

.state-error {
  border: 1px solid var(--accent-400);

  .state-label {
    color: var(--accent-500);
  }
}

.spinner {
  width: 2rem;
  height: 2rem;
  border: 3px solid rgba(0, 0, 0, 0.1);
  border-top-color: var(--accent-500);
  border-radius: 50%;
  animation: spin 0.8s linear infinite;
}

.spinner-small {
  width: 1rem;
  height: 1rem;
  border: 2px solid rgba(0, 0, 0, 0.1);
  border-top-color: var(--accent-500);
  border-radius: 50%;
  animation: spin 0.8s linear infinite;
}

@keyframes spin {
  to {
    transform: rotate(360deg);
  }
}
</style>
