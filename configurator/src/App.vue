<script setup lang="ts">
import { watch, ref, computed } from "vue"
import { RouterView, useRoute } from "vue-router"
import AppSidebar from "@/components/AppSidebar.vue"
import { provideSurreal } from "@/lib/surreal"
import { authReady, tryRestoreSession } from "@/lib/auth"

const route = useRoute()
const showApp = ref(false)
const connectionError = ref<string | null>(null)
const isAuthPage = computed(() =>
  ["login", "register"].includes(String(route.name))
)

const { client, isSuccess, isError, connect, error } = provideSurreal({
  endpoint: "ws://192.168.10.87:8090",
  params: {
    namespace: "remex",
    database: "remex",
  },
  autoConnect: false,
})

connect().catch(() => {})

watch(isSuccess, async (connected) => {
  if (connected) {
    await tryRestoreSession(client)
  }
})

watch(isError, (errored) => {
  if (errored) {
    connectionError.value = String(error.value ?? "Connection failed")
  }
})

watch(authReady, (ready) => {
  if (ready) {
    showApp.value = true
  }
})
</script>

<template>
  <div v-if="connectionError" class="loading-screen">
    <div class="error-icon">⚠</div>
    <p class="error-title">Connection failed</p>
    <p class="error-detail">{{ connectionError }}</p>
  </div>

  <div v-else-if="!showApp" class="loading-screen">
    <div class="spinner" />
    <p class="loading-text">Connecting to database...</p>
  </div>

  <div v-else-if="!isAuthPage" class="app-layout">
    <AppSidebar />
    <main class="main-content">
      <RouterView />
    </main>
  </div>

  <div v-else class="auth-layout">
    <RouterView />
  </div>
</template>

<style lang="scss">
*,
*::before,
*::after {
  box-sizing: border-box;
  margin: 0;
  font-weight: normal;
}

body {
  margin: 0;
  background: var(--background);
  color: var(--text);
  transition: color 0.3s, background-color 0.3s;
  font-family: Inter, -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto,
    Oxygen, Ubuntu, Cantarell, "Fira Sans", "Droid Sans", "Helvetica Neue",
    sans-serif;
  font-size: 15px;
  text-rendering: optimizeLegibility;
  -webkit-font-smoothing: antialiased;
  -moz-osx-font-smoothing: grayscale;
}

@media (prefers-color-scheme: light) {
  :root {
    --text: #0c1f2c;
    --background: #dadee2;
    --primary: #36414a;
    --secondary: #8da0bf;
    --accent: #ff932e;

    --text-50: #f0f2f5;
    --text-100: #e1e6ea;
    --text-200: #c3ccd5;
    --text-300: #a5b3c0;
    --text-400: #8799ab;
    --text-500: #697f96;
    --text-600: #546678;
    --text-700: #3f4c5a;
    --text-800: #2a333c;
    --text-900: #15191e;
    --text-950: #0a0d0f;

    --background-50: #f1f2f3;
    --background-100: #e3e6e8;
    --background-200: #c7ccd1;
    --background-300: #acb3b9;
    --background-400: #9099a2;
    --background-500: #747f8b;
    --background-600: #5d666f;
    --background-700: #464c53;
    --background-800: #2e3338;
    --background-900: #171a1c;
    --background-950: #0c0d0e;

    --primary-50: #f0f2f4;
    --primary-100: #e1e6ea;
    --primary-200: #c4cdd4;
    --primary-300: #a6b4bf;
    --primary-400: #899ba9;
    --primary-500: #6b8294;
    --primary-600: #566876;
    --primary-700: #404e59;
    --primary-800: #2b343b;
    --primary-900: #151a1e;
    --primary-950: #0b0d0f;

    --secondary-50: #eff1f6;
    --secondary-100: #dee4ed;
    --secondary-200: #bec9da;
    --secondary-300: #9daec8;
    --secondary-400: #7c92b6;
    --secondary-500: #5c77a3;
    --secondary-600: #495f83;
    --secondary-700: #374862;
    --secondary-800: #253041;
    --secondary-900: #121821;
    --secondary-950: #090c10;

    --accent-50: #fff2e5;
    --accent-100: #ffe5cc;
    --accent-200: #ffca99;
    --accent-300: #ffb066;
    --accent-400: #ff9633;
    --accent-500: #ff7b00;
    --accent-600: #cc6300;
    --accent-700: #994a00;
    --accent-800: #663100;
    --accent-900: #331900;
    --accent-950: #1a0c00;

    --status-pending-bg: #fef9c3;
    --status-pending-text: #854d0e;
    --status-running-bg: #dcfce7;
    --status-running-text: #166534;
    --status-completed-bg: #dcfce7;
    --status-completed-text: #166534;
    --status-failed-bg: #fee2e2;
    --status-failed-text: #991b1b;
    --status-timedout-bg: #f3e8ff;
    --status-timedout-text: #6b21a8;

    --danger-bg: #fee2e2;
    --danger-text: #991b1b;
  }
}

@media (prefers-color-scheme: dark) {
  :root {
    --text: #d4e6f3;
    --background: #1d2226;
    --primary: #b5c0c9;
    --secondary: #405372;
    --accent: #d36600;

    --text-50: #0a0d0f;
    --text-100: #15191e;
    --text-200: #29323d;
    --text-300: #3e4c5b;
    --text-400: #536579;
    --text-500: #677e98;
    --text-600: #8698ac;
    --text-700: #a4b2c1;
    --text-800: #c2cbd6;
    --text-900: #e1e5ea;
    --text-950: #f0f2f5;

    --background-50: #0c0d0e;
    --background-100: #171a1c;
    --background-200: #2e3338;
    --background-300: #464c53;
    --background-400: #5d666f;
    --background-500: #747f8b;
    --background-600: #9099a2;
    --background-700: #acb3b9;
    --background-800: #c7ccd1;
    --background-900: #e3e6e8;
    --background-950: #f1f2f3;

    --primary-50: #0b0d0f;
    --primary-100: #151a1e;
    --primary-200: #2b343b;
    --primary-300: #404e59;
    --primary-400: #566876;
    --primary-500: #6b8294;
    --primary-600: #899ba9;
    --primary-700: #a6b4bf;
    --primary-800: #c4cdd4;
    --primary-900: #e1e6ea;
    --primary-950: #f0f2f4;

    --secondary-50: #090c10;
    --secondary-100: #121821;
    --secondary-200: #253041;
    --secondary-300: #374862;
    --secondary-400: #495f83;
    --secondary-500: #5c77a3;
    --secondary-600: #7c92b6;
    --secondary-700: #9daec8;
    --secondary-800: #bec9da;
    --secondary-900: #dee4ed;
    --secondary-950: #eff1f6;

    --accent-50: #1a0c00;
    --accent-100: #331900;
    --accent-200: #663100;
    --accent-300: #994a00;
    --accent-400: #cc6300;
    --accent-500: #ff7b00;
    --accent-600: #ff9633;
    --accent-700: #ffb066;
    --accent-800: #ffca99;
    --accent-900: #ffe5cc;
    --accent-950: #fff2e5;

    --status-pending-bg: #422006;
    --status-pending-text: #fbbf24;
    --status-running-bg: #052e16;
    --status-running-text: #22c55e;
    --status-completed-bg: #052e16;
    --status-completed-text: #22c55e;
    --status-failed-bg: #450a0a;
    --status-failed-text: #ef4444;
    --status-timedout-bg: #3b0764;
    --status-timedout-text: #d8b4fe;

    --danger-bg: #450a0a;
    --danger-text: #fca5a5;
  }
}

:root {
  --color-background: var(--background);
}

.app-layout {
  display: flex;
  height: 100vh;
  overflow: hidden;
}

.main-content {
  flex: 1;
  overflow-y: auto;
  padding: 0;
}

.auth-layout {
  display: flex;
  align-items: center;
  justify-content: center;
  height: 100vh;
  background: var(--background);
}

.loading-screen {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  height: 100vh;
  gap: 1rem;
  background: var(--background);
  color: var(--text);
}

.spinner {
  width: 2.5rem;
  height: 2.5rem;
  border: 3px solid var(--primary-300);
  border-top-color: var(--accent-500);
  border-radius: 50%;
  animation: spin 0.8s linear infinite;
}

@keyframes spin {
  to {
    transform: rotate(360deg);
  }
}

.loading-text {
  font-size: 1rem;
  opacity: 0.7;
}

.error-icon {
  font-size: 3rem;
}

.error-title {
  font-size: 1.25rem;
  font-weight: 700;
}

.error-detail {
  font-size: 0.9rem;
  opacity: 0.7;
}
</style>
