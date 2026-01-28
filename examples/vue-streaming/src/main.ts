import { createApp } from 'vue'
import { BgqlPlugin } from '@bgql/client/vue'
import App from './App.vue'

const app = createApp(App)

app.use(BgqlPlugin, {
  endpoint: '/graphql',
})

app.mount('#app')
