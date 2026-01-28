import { createApp } from 'vue'
import '@mdi/font/css/materialdesignicons.css'
import './styles/main.scss'
import App from './App.vue'

// Configure Monaco Editor workers
import editorWorker from 'monaco-editor/esm/vs/editor/editor.worker?worker'

self.MonacoEnvironment = {
  getWorker() {
    return new editorWorker()
  },
}

createApp(App).mount('#app')
