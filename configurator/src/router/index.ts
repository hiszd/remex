import { watch } from "vue"
import { createRouter, createWebHistory } from "vue-router"
import { authReady, isAuthenticatedGlob } from "@/lib/auth"
import DashboardView from "@/views/DashboardView.vue"
import ClientsView from "@/views/ClientsView.vue"
import ClientDetailsView from "@/views/ClientDetailsView.vue"
import JobsView from "@/views/JobsView.vue"
import CreateJobView from "@/views/CreateJobView.vue"
import JobDetailsView from "@/views/JobDetailsView.vue"
import GroupsView from "@/views/GroupsView.vue"
import GroupDetailsView from "@/views/GroupDetailsView.vue"
import CreateGroupView from "@/views/CreateGroupView.vue"
import LoginView from "@/views/LoginView.vue"
import RegisterView from "@/views/RegisterView.vue"

const router = createRouter({
  history: createWebHistory(import.meta.env.BASE_URL),
  routes: [
    {
      path: "/login",
      name: "login",
      component: LoginView,
    },
    {
      path: "/register",
      name: "register",
      component: RegisterView,
    },
    {
      path: "/",
      name: "home",
      component: DashboardView,
    },
    {
      path: "/clients",
      name: "clients",
      component: ClientsView,
    },
    {
      path: "/clients/:id",
      name: "client-details",
      component: ClientDetailsView,
    },
    {
      path: "/groups",
      name: "groups",
      component: GroupsView,
    },
    {
      path: "/groups/:id",
      name: "group-details",
      component: GroupDetailsView,
    },
    {
      path: "/groups/new",
      name: "create-group",
      component: CreateGroupView,
    },
    {
      path: "/jobs",
      name: "jobs",
      component: JobsView,
    },
    {
      path: "/jobs/new",
      name: "create-job",
      component: CreateJobView,
    },
    {
      path: "/jobs/:id",
      name: "job-details",
      component: JobDetailsView,
    },
  ],
})

const publicRoutes = ["login", "register"]

router.beforeEach(async (to) => {
  if (!authReady.value) {
    await new Promise<void>((resolve) => {
      const unwatch = watch(authReady, (ready) => {
        if (ready) {
          unwatch()
          resolve()
        }
      })
    })
  }

  if (!isAuthenticatedGlob.value && !publicRoutes.includes(String(to.name))) {
    return { name: "login" }
  }

  if (isAuthenticatedGlob.value && publicRoutes.includes(String(to.name))) {
    return { name: "home" }
  }
})

export default router
