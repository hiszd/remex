import { markRaw, watch } from "vue"
import { createRouter, createWebHistory } from "vue-router"
import { authReady, isAuthenticatedGlob } from "@/lib/auth"

const router = markRaw(createRouter({
  history: createWebHistory(import.meta.env.BASE_URL),
  routes: [
    {
      path: "/login",
      name: "login",
      component: () => import("@/views/LoginView.vue"),
    },
    {
      path: "/register",
      name: "register",
      component: () => import("@/views/RegisterView.vue"),
    },
    {
      path: "/",
      name: "home",
      meta: { title: "Dashboard" },
      component: () => import("@/views/DashboardView.vue"),
    },
    {
      path: "/clients",
      name: "clients",
      meta: { title: "Clients" },
      component: () => import("@/views/ClientsView.vue"),
    },
    {
      path: "/clients/:id",
      name: "client-details",
      meta: { title: "Client Details" },
      component: () => import("@/views/ClientDetailsView.vue"),
    },
    {
      path: "/groups",
      name: "groups",
      meta: { title: "Groups" },
      component: () => import("@/views/GroupsView.vue"),
    },
    {
      path: "/groups/:id",
      name: "group-details",
      meta: { title: "Group Details" },
      component: () => import("@/views/GroupDetailsView.vue"),
    },
    {
      path: "/groups/new",
      name: "create-group",
      meta: { title: "Create Group" },
      component: () => import("@/views/CreateGroupView.vue"),
    },
    {
      path: "/jobs",
      name: "jobs",
      meta: { title: "Jobs" },
      component: () => import("@/views/JobsView.vue"),
    },
    {
      path: "/jobs/new",
      name: "create-job",
      meta: { title: "Create Job" },
      component: () => import("@/views/CreateJobView.vue"),
    },
    {
      path: "/jobs/:id",
      name: "job-details",
      meta: { title: "Job Details" },
      component: () => import("@/views/JobDetailsView.vue"),
    },
    {
      path: "/tokens",
      name: "tokens",
      meta: { title: "Enrollment Tokens" },
      component: () => import("@/views/TokensView.vue"),
    },
    {
      path: "/tokens/new",
      name: "create-token",
      meta: { title: "Generate Token" },
      component: () => import("@/views/CreateTokenView.vue"),
    },
    {
      path: "/tokens/:id",
      name: "token-details",
      meta: { title: "Token Details" },
      component: () => import("@/views/TokenDetailsView.vue"),
    },
    {
      path: "/executions/:id",
      name: "execution-details",
      meta: { title: "Execution Details" },
      component: () => import("@/views/ExecutionDetailsView.vue"),
    },
  ],
}))

const publicRoutes = ["login", "register"]

router.beforeEach(async (to) => {
  if (!authReady.value) {
    await new Promise<void>((resolve) => {
      watch(authReady, (ready) => {
        if (ready) resolve()
      }, { once: true })
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
