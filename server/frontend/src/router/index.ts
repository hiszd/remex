import { createRouter, createWebHistory } from 'vue-router'
import HomeView from '@/views/HomeView.vue'
import ClientsView from '@/views/ClientsView.vue'
import JobsView from '@/views/JobsView.vue'
import CreateJobView from '@/views/CreateJobView.vue'
import JobDetailsView from '@/views/JobDetailsView.vue'

const router = createRouter({
  history: createWebHistory(import.meta.env.BASE_URL),
  routes: [
    {
      path: '/',
      name: 'home',
      component: HomeView,
    },
    {
      path: '/clients',
      name: 'clients',
      component: ClientsView,
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
