<script setup lang="ts">
import { shallowRef, computed } from "vue"
import { useRouter } from "vue-router"
import DesignSidebar from "@/components/design/DesignSidebar.vue"
import DesignTopBar from "@/components/design/DesignTopBar.vue"

const router = useRouter()

const sidebarExpanded = shallowRef(true)
const currentView = shallowRef<"dashboard" | "jobs" | "job-detail">("dashboard")
const selectedJobId = shallowRef<string | null>(null)
const viewMode = shallowRef<"table" | "grid">("table")

interface MockJob {
  id: string
  job_name: string
  job_shell: string
  job_command: string
  execution_status: string
  enabled: string
  timeout: number
  created_at: string
  client_name?: string
  exit_code?: number | null
}

const mockJobs = shallowRef<MockJob[]>([
  { id: "job:001", job_name: "System Backup", job_shell: "/bin/bash", job_command: "./scripts/backup.sh --full", execution_status: "Completed", enabled: "Enabled", timeout: 300, created_at: "2026-06-20T08:00:00Z", client_name: "server-prod-01", exit_code: 0 },
  { id: "job:002", job_name: "Health Check", job_shell: "/bin/sh", job_command: "curl -f http://localhost:8080/health", execution_status: "Running", enabled: "Enabled", timeout: 30, created_at: "2026-06-19T14:30:00Z", client_name: "server-prod-02" },
  { id: "job:003", job_name: "Database Cleanup", job_shell: "/bin/bash", job_command: "psql -c 'VACUUM ANALYZE;'", execution_status: "Failed", enabled: "Enabled", timeout: 600, created_at: "2026-06-18T22:00:00Z", client_name: "db-primary", exit_code: 1 },
  { id: "job:004", job_name: "Log Rotation", job_shell: "/bin/bash", job_command: "logrotate -f /etc/logrotate.conf", execution_status: "Pending", enabled: "Draft", timeout: 120, created_at: "2026-06-17T06:00:00Z" },
  { id: "job:005", job_name: "Deploy Frontend", job_shell: "/bin/bash", job_command: "ansible-playbook deploy-frontend.yml", execution_status: "TimedOut", enabled: "Enabled", timeout: 900, created_at: "2026-06-16T10:00:00Z", client_name: "web-cluster-01", exit_code: null },
  { id: "job:006", job_name: "Certificate Renewal", job_shell: "/bin/bash", job_command: "certbot renew --quiet", execution_status: "Completed", enabled: "Disabled", timeout: 120, created_at: "2026-06-15T03:00:00Z", client_name: "web-cluster-01", exit_code: 0 },
  { id: "job:007", job_name: "Data Sync", job_shell: "/bin/sh", job_command: "rsync -avz /data /backup", execution_status: "Completed", enabled: "Enabled", timeout: 600, created_at: "2026-06-14T02:00:00Z", client_name: "db-secondary", exit_code: 0 },
  { id: "job:008", job_name: "Security Scan", job_shell: "/bin/bash", job_command: "nmap -sV 192.168.1.0/24", execution_status: "Pending", enabled: "Draft", timeout: 3600, created_at: "2026-06-13T12:00:00Z" },
])

const stats = shallowRef({
  totalJobs: 8,
  activeGroups: 4,
  onlineClients: 12,
  executionsToday: 47,
  jobsChange: "+12%",
  groupsChange: "+1",
  clientsChange: "+2",
  executionsChange: "+23%",
})

const selectedJob = computed(() =>
  mockJobs.value.find(j => j.id === selectedJobId.value) ?? null
)

function navigate(path: string) {
  if (path === "/design") currentView.value = "dashboard"
  else if (path === "/design/jobs") currentView.value = "jobs"
}

function viewJob(jobId: string) {
  selectedJobId.value = jobId
  currentView.value = "job-detail"
}

function goBack() {
  currentView.value = "jobs"
  selectedJobId.value = null
}
</script>

<template>
  <div class="design-layout">
    <DesignSidebar
      :expanded="sidebarExpanded"
      :active-item="currentView === 'dashboard' ? '/design' : '/design/jobs'"
      @update:expanded="sidebarExpanded = $event"
      @navigate="navigate"
      @logout="router.push('/login')"
    />

    <div class="design-content">
      <!-- ===== DASHBOARD ===== -->
      <template v-if="currentView === 'dashboard'">
        <DesignTopBar
          title="Dashboard"
          :breadcrumbs="[{ label: 'Dashboard' }]"
          :show-search="true"
          @toggle-sidebar="sidebarExpanded = !sidebarExpanded"
        />
        <div class="design-main">
          <div class="stats-grid">
            <div class="stat-card">
              <div class="stat-card-header">
                <span class="stat-card-label">Total Jobs</span>
                <div class="stat-icon">
                  <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                    <rect width="20" height="14" x="2" y="7" rx="2" ry="2" /><path d="M16 21V5a2 2 0 0 0-2-2h-4a2 2 0 0 0-2 2v16" />
                  </svg>
                </div>
              </div>
              <span class="stat-card-value">{{ stats.totalJobs }}</span>
              <span class="stat-card-change up">{{ stats.jobsChange }} from last week</span>
            </div>
            <div class="stat-card">
              <div class="stat-card-header">
                <span class="stat-card-label">Active Groups</span>
                <div class="stat-icon">
                  <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                    <path d="M16 21v-2a4 4 0 0 0-4-4H6a4 4 0 0 0-4 4v2" /><circle cx="9" cy="7" r="4" /><path d="M22 21v-2a4 4 0 0 0-3-3.87" /><path d="M16 3.13a4 4 0 0 1 0 7.75" />
                  </svg>
                </div>
              </div>
              <span class="stat-card-value">{{ stats.activeGroups }}</span>
              <span class="stat-card-change up">{{ stats.groupsChange }} from last week</span>
            </div>
            <div class="stat-card">
              <div class="stat-card-header">
                <span class="stat-card-label">Online Endpoints</span>
                <div class="stat-icon">
                  <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                    <rect width="14" height="8" x="5" y="2" rx="2" />
                    <rect width="20" height="8" x="2" y="14" rx="2" />
                    <path d="M6 18h2" />
                    <path d="M12 18h6" />
                  </svg>
                </div>
              </div>
              <span class="stat-card-value">{{ stats.onlineClients }}</span>
              <span class="stat-card-change up">{{ stats.clientsChange }} from last week</span>
            </div>
            <div class="stat-card">
              <div class="stat-card-header">
                <span class="stat-card-label">Executions Today</span>
                <div class="stat-icon">
                  <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                    <polygon points="13 2 3 14 12 14 11 22 21 10 12 10" />
                  </svg>
                </div>
              </div>
              <span class="stat-card-value">{{ stats.executionsToday }}</span>
              <span class="stat-card-change up">{{ stats.executionsChange }} from yesterday</span>
            </div>
          </div>

          <div style="margin-top: 2rem">
            <div class="section-header-row">
              <h2>Recent Executions</h2>
              <div class="section-actions">
                <button class="btn-ghost" style="padding: 0.5rem 0.75rem; font-size: 0.8rem">View All</button>
              </div>
            </div>
            <div class="data-table-card">
              <table class="data-table">
                <thead>
                  <tr>
                    <th>Job</th>
                    <th>Client</th>
                    <th>Status</th>
                    <th>Duration</th>
                    <th>Started</th>
                  </tr>
                </thead>
                <tbody>
                  <tr v-for="job in mockJobs.slice(0, 5)" :key="job.id">
                    <td style="font-weight: 600; color: var(--text-900)">{{ job.job_name }}</td>
                    <td>{{ job.client_name ?? "—" }}</td>
                    <td><span class="status-badge" :class="job.execution_status.toLowerCase()">{{ job.execution_status }}</span></td>
                    <td>{{ job.timeout }}s</td>
                    <td style="color: var(--text-500); font-size: 0.8rem">{{ job.created_at }}</td>
                  </tr>
                </tbody>
              </table>
            </div>
          </div>
        </div>
      </template>

      <!-- ===== JOBS LIST ===== -->
      <template v-else-if="currentView === 'jobs'">
        <DesignTopBar
          title="Jobs"
          :breadcrumbs="[{ label: 'Jobs' }]"
          :show-search="true"
          @toggle-sidebar="sidebarExpanded = !sidebarExpanded"
        />
        <div class="design-main">
          <div class="section-header-row">
            <span style="font-size: 0.875rem; color: var(--text-500)">{{ mockJobs.length }} jobs total</span>
            <div class="section-actions">
              <div class="view-toggle">
                <button :class="{ active: viewMode === 'table' }" @click="viewMode = 'table'" title="Table view">
                  <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                    <rect x="3" y="3" width="7" height="7" /><rect x="14" y="3" width="7" height="7" /><rect x="14" y="14" width="7" height="7" /><rect x="3" y="14" width="7" height="7" />
                  </svg>
                </button>
                <button :class="{ active: viewMode === 'grid' }" @click="viewMode = 'grid'" title="Grid view">
                  <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                    <rect x="3" y="3" width="18" height="18" rx="2" /><line x1="3" y1="9" x2="21" y2="9" /><line x1="9" y1="21" x2="9" y2="9" />
                  </svg>
                </button>
              </div>
              <button class="btn-primary" style="padding: 0.5rem 1rem; font-size: 0.8rem">+ New Job</button>
            </div>
          </div>

          <!-- Table View -->
          <div v-if="viewMode === 'table'" class="data-table-card">
            <table class="data-table">
              <thead>
                <tr>
                  <th class="sortable">Name</th>
                  <th>Status</th>
                  <th>State</th>
                  <th>Shell</th>
                  <th>Timeout</th>
                  <th>Created</th>
                  <th style="text-align: right">Actions</th>
                </tr>
              </thead>
              <tbody>
                <tr v-for="job in mockJobs" :key="job.id" style="cursor: pointer" @click="viewJob(job.id)">
                  <td style="font-weight: 600; color: var(--text-900)">{{ job.job_name }}</td>
                  <td><span class="status-badge" :class="job.execution_status.toLowerCase()">{{ job.execution_status }}</span></td>
                  <td><span class="state-badge" :class="job.enabled.toLowerCase()">{{ job.enabled }}</span></td>
                  <td style="font-family: monospace; font-size: 0.8rem">{{ job.job_shell }}</td>
                  <td>{{ job.timeout }}s</td>
                  <td style="color: var(--text-500); font-size: 0.8rem">{{ job.created_at }}</td>
                  <td style="text-align: right">
                    <button class="btn-ghost" style="padding: 0.25rem 0.5rem; font-size: 0.75rem" @click.stop="viewJob(job.id)">Details</button>
                  </td>
                </tr>
              </tbody>
            </table>
          </div>

          <!-- Grid View -->
          <div v-else class="design-job-grid">
            <div v-for="job in mockJobs" :key="job.id" class="design-job-card" @click="viewJob(job.id)">
              <div class="job-card-top">
                <span class="job-card-name">{{ job.job_name }}</span>
                <span class="status-badge" :class="job.execution_status.toLowerCase()">{{ job.execution_status }}</span>
              </div>
              <div class="job-card-meta">
                <div class="job-card-field">
                  <span class="job-card-label">Shell</span>
                  <span class="job-card-value" style="font-family: monospace">{{ job.job_shell }}</span>
                </div>
                <div class="job-card-field">
                  <span class="job-card-label">State</span>
                  <span class="state-badge" :class="job.enabled.toLowerCase()">{{ job.enabled }}</span>
                </div>
                <div class="job-card-field">
                  <span class="job-card-label">Timeout</span>
                  <span class="job-card-value">{{ job.timeout }}s</span>
                </div>
                <div class="job-card-field">
                  <span class="job-card-label">Created</span>
                  <span class="job-card-value" style="font-size: 0.75rem">{{ job.created_at }}</span>
                </div>
              </div>
            </div>
          </div>
        </div>
      </template>

      <!-- ===== JOB DETAIL ===== -->
      <template v-else-if="currentView === 'job-detail' && selectedJob">
        <DesignTopBar
          :title="selectedJob.job_name"
          :breadcrumbs="[
            { label: 'Jobs', path: '/design/jobs' },
            { label: selectedJob.job_name }
          ]"
          @toggle-sidebar="sidebarExpanded = !sidebarExpanded"
        />
        <div class="design-main">
          <button class="back-link" @click="goBack">← Back to Jobs</button>

          <div class="card" style="margin-bottom: 1.5rem">
            <div class="header-main" style="margin-bottom: 1.5rem">
              <div class="title-group">
                <h1 class="page-title">{{ selectedJob.job_name }}</h1>
                <p class="record-id">{{ selectedJob.id }}</p>
              </div>
              <div class="header-actions">
                <button class="btn-primary" style="padding: 0.5rem 1rem; font-size: 0.8rem">Edit</button>
                <button class="btn-danger" style="padding: 0.5rem 1rem; font-size: 0.8rem">Delete</button>
              </div>
            </div>

            <div class="info-grid">
              <div class="info-item">
                <span class="label">Status</span>
                <span class="status-badge" :class="selectedJob.execution_status.toLowerCase()">{{ selectedJob.execution_status }}</span>
              </div>
              <div class="info-item">
                <span class="label">State</span>
                <span class="state-badge" :class="selectedJob.enabled.toLowerCase()">{{ selectedJob.enabled }}</span>
              </div>
              <div class="info-item">
                <span class="label">Shell</span>
                <span class="value monospace">{{ selectedJob.job_shell }}</span>
              </div>
              <div class="info-item">
                <span class="label">Timeout</span>
                <span class="value">{{ selectedJob.timeout }} seconds</span>
              </div>
              <div class="info-item">
                <span class="label">Created</span>
                <span class="value">{{ selectedJob.created_at }}</span>
              </div>
              <div class="info-item">
                <span class="label">Client</span>
                <span class="value">{{ selectedJob.client_name ?? "Not assigned" }}</span>
              </div>
            </div>
          </div>

          <div class="card">
            <h2 style="font-size: 1.125rem; font-weight: 700; margin-bottom: 0.75rem">Command</h2>
            <pre class="command-block">{{ selectedJob.job_command }}</pre>
          </div>

          <div style="margin-top: 1.5rem">
            <div class="section-header-row">
              <h2>Execution History</h2>
            </div>
            <div class="data-table-card">
              <table class="data-table">
                <thead>
                  <tr>
                    <th>Status</th>
                    <th>Client</th>
                    <th>Exit Code</th>
                    <th>Started</th>
                    <th>Duration</th>
                  </tr>
                </thead>
                <tbody>
                  <tr>
                    <td><span class="status-badge" :class="selectedJob.execution_status.toLowerCase()">{{ selectedJob.execution_status }}</span></td>
                    <td>{{ selectedJob.client_name ?? "—" }}</td>
                    <td>{{ selectedJob.exit_code ?? "—" }}</td>
                    <td style="color: var(--text-500); font-size: 0.8rem">{{ selectedJob.created_at }}</td>
                    <td>{{ selectedJob.timeout }}s</td>
                  </tr>
                </tbody>
              </table>
            </div>
          </div>
        </div>
      </template>
    </div>
  </div>
</template>

<style scoped>
.design-layout {
  flex: 1;
  width: 100%;
}

.design-job-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(300px, 1fr));
  gap: 1rem;
}

.design-job-card {
  background: var(--background-50);
  border: 1px solid var(--background-400);
  border-radius: 0.75rem;
  padding: 1rem;
  cursor: pointer;
  transition: border-color 0.15s, box-shadow 0.15s;

  &:hover {
    border-color: var(--accent-300);
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.06);
  }
}

.job-card-top {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 0.75rem;
  gap: 0.5rem;
}

.job-card-name {
  font-weight: 700;
  font-size: 0.95rem;
  color: var(--text-900);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.job-card-meta {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 0.5rem;
}

.job-card-field {
  display: flex;
  flex-direction: column;
  gap: 0.125rem;
}

.job-card-label {
  font-size: 0.65rem;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 0.05em;
  color: var(--text-500);
}

.job-card-value {
  font-size: 0.8rem;
  color: var(--text-800);
}

.command-block {
  background: var(--background-950);
  color: var(--text-100);
  padding: 1rem;
  border-radius: 0.5rem;
  font-family: ui-monospace, monospace;
  font-size: 0.85rem;
  overflow-x: auto;
  white-space: pre-wrap;
  word-break: break-all;
}

@media (min-width: 1024px) {
  .design-job-grid {
    grid-template-columns: repeat(auto-fill, minmax(360px, 1fr));
  }
}

@media (min-width: 1440px) {
  .design-job-grid {
    grid-template-columns: repeat(auto-fill, minmax(400px, 1fr));
  }
}
</style>
