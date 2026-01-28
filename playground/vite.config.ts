import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'
import { resolve } from 'path'

export default defineConfig({
  plugins: [vue()],
  base: '/better-graphql/',
  build: {
    target: 'esnext',
    outDir: 'dist',
  },
  optimizeDeps: {
    include: ['monaco-editor'],
    exclude: ['bgql_wasm'],
  },
  worker: {
    format: 'es',
  },
  resolve: {
    alias: {
      'bgql_wasm': resolve(__dirname, '../crates/bgql_wasm/pkg'),
    },
  },
  server: {
    fs: {
      allow: ['..'],
    },
  },
  assetsInclude: ['**/*.wasm'],
})
