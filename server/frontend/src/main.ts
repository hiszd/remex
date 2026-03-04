import './assets/main.css'

import { createApp } from 'vue'
import App from './App.vue'
import router from './router'
import { VueQueryPlugin } from '@tanstack/vue-query'
import { client } from '@/client/client.gen';

client.setConfig({
  baseUrl: '/api',
})

const app = createApp(App)

app.use(router)

// Install the VueQueryPlugin
// You can pass the custom queryClient instance as an option
app.use(VueQueryPlugin);

app.mount('#app')
