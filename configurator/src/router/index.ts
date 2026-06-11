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
      component: () => import("@/views/DashboardView.vue"),
    },
    {
      path: "/clients",
      name: "clients",
      component: () => import("@/views/ClientsView.vue"),
    },
    {
      path: "/clients/:id",
      name: "client-details",
      component: () => import("@/views/ClientDetailsView.vue"),
    },
    {
      path: "/groups",
      name: "groups",
      component: () => import("@/views/GroupsView.vue"),
    },
    {
      path: "/groups/:id",
      name: "group-details",
      component: () => import("@/views/GroupDetailsView.vue"),
    },
    {
      path: "/groups/new",
      name: "create-group",
      component: () => import("@/views/CreateGroupView.vue"),
    },
    {
      path: "/jobs",
      name: "jobs",
      component: () => import("@/views/JobsView.vue"),
    },
    {
      path: "/jobs/new",
      name: "create-job",
      component: () => import("@/views/CreateJobView.vue"),
    },
    {
      path: "/jobs/:id",
      name: "job-details",
      component: () => import("@/views/JobDetailsView.vue"),
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
