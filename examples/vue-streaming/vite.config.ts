import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'
import { bgqlPlugin } from '@bgql/client/vue'

export default defineConfig({
  plugins: [
    vue(),
    bgqlPlugin({
      endpoint: '/graphql',
      wsEndpoint: '/graphql/ws',
      binaryEndpoint: '/graphql/binary',
      ssr: true,
      cacheStrategy: 'request',
      dev: {
        playground: true,
        logging: true,
      },
    }),
  ],
})
