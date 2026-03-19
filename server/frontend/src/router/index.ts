import { createRouter, createWebHistory } from 'vue-router'
import DashboardView from '@/views/DashboardView.vue'
import ClientsView from '@/views/ClientsView.vue'
import ClientDetailsView from '@/views/ClientDetailsView.vue'
import JobsView from '@/views/JobsView.vue'
import CreateJobView from '@/views/CreateJobView.vue'
import JobDetailsView from '@/views/JobDetailsView.vue'
import GroupsView from '@/views/GroupsView.vue'
import GroupDetailsView from '@/views/GroupDetailsView.vue'
import CreateGroupView from '@/views/CreateGroupView.vue'

const router = createRouter({
  history: createWebHistory(import.meta.env.BASE_URL),
  routes: [
    {
      path: '/',
      name: 'home',
      component: DashboardView,
    },
    {
      path: '/clients',
      name: 'clients',
      component: ClientsView,
    },
    {
      path: '/clients/:id',
      name: 'client-details',
      component: ClientDetailsView,
    },
    {
      path: '/groups',
      name: 'groups',
      component: GroupsView,
    },
    {
      path: '/groups/:id',
      name: 'group-details',
      component: GroupDetailsView,
    },
    {
      path: '/groups/new',
      name: 'create-group',
      component: CreateGroupView,
    },
    {
      path: '/jobs',
      name: 'jobs',
      component: JobsView,
    },
    {
      path: '/jobs/new',
      name: 'create-job',
      component: CreateJobView,
    },
    {
      path: '/jobs/:id',
      name: 'job-details',
      component: JobDetailsView,
    }
    // {
    //   path: '/clients',
    //   name: 'clients',
    //   // route level code-splitting
    //   // this generates a separate chunk (About.[hash].js) for this route
    //   // which is lazy-loaded when the route is visited.
    //   component: () => import('@/views/ClientsView.vue'),
    // },
  ],
})

export default router
