<script setup lang="ts">
import { ref, computed } from 'vue'

const props = defineProps<{
  output: string
  errors: Array<{ message: string; line: number; column: number }>
  isProcessing: boolean
}>()

type Tab = 'output' | 'ast' | 'diagnostics' | 'types'
const activeTab = ref<Tab>('output')

const tabs: Array<{ id: Tab; label: string; icon: string }> = [
  { id: 'output', label: 'Output', icon: 'mdi-console' },
  { id: 'ast', label: 'AST', icon: 'mdi-file-tree' },
  { id: 'diagnostics', label: 'Diagnostics', icon: 'mdi-alert-circle-outline' },
  { id: 'types', label: 'Types', icon: 'mdi-shape-outline' },
]

const formattedOutput = computed(() => {
  if (!props.output) return ''

  try {
    const obj = JSON.parse(props.output)
    return JSON.stringify(obj, null, 2)
  } catch {
    return props.output
  }
})

const syntaxHighlight = (json: string) => {
  return json.replace(
    /("(\\u[a-zA-Z0-9]{4}|\\[^u]|[^\\"])*"(\s*:)?|\b(true|false|null)\b|-?\d+(?:\.\d*)?(?:[eE][+\-]?\d+)?)/g,
    (match) => {
      let cls = 'number'
      if (/^"/.test(match)) {
        if (/:$/.test(match)) {
          cls = 'key'
        } else {
          cls = 'string'
        }
      } else if (/true|false/.test(match)) {
        cls = 'boolean'
      } else if (/null/.test(match)) {
        cls = 'null'
      }
      return `<span class="${cls}">${match}</span>`
    }
  )
}
</script>

<template>
  <div class="panel">
    <div class="tabs">
      <button
        v-for="tab in tabs"
        :key="tab.id"
        :class="['tab', { active: activeTab === tab.id }]"
        @click="activeTab = tab.id"
      >
        <i :class="['mdi', tab.icon]"></i>
        {{ tab.label }}
        <span v-if="tab.id === 'diagnostics' && errors.length" class="count">
          {{ errors.length }}
        </span>
      </button>
    </div>

    <div class="panel__content">
      <!-- Output Tab -->
      <div v-if="activeTab === 'output'" class="output output--json">
        <div v-if="isProcessing" style="display: flex; align-items: center; gap: 8px; padding: 12px;">
          <div class="spinner"></div>
          Processing...
        </div>
        <pre v-else v-html="syntaxHighlight(formattedOutput)"></pre>
      </div>

      <!-- AST Tab -->
      <div v-else-if="activeTab === 'ast'" class="output">
        <div style="color: var(--text-muted); padding: 24px; text-align: center;">
          <i class="mdi mdi-file-tree" style="font-size: 48px; opacity: 0.3;"></i>
          <p style="margin-top: 12px;">AST visualization coming soon</p>
        </div>
      </div>

      <!-- Diagnostics Tab -->
      <div v-else-if="activeTab === 'diagnostics'" class="diagnostics">
        <div v-if="errors.length === 0" class="diagnostics__empty">
          <i class="mdi mdi-check-circle-outline"></i>
          <span>No issues found</span>
        </div>
        <div v-else>
          <div
            v-for="(error, i) in errors"
            :key="i"
            class="diagnostics__item"
          >
            <div class="icon error">
              <i class="mdi mdi-close-circle"></i>
            </div>
            <div class="content">
              <div class="message">{{ error.message }}</div>
              <div class="location">Line {{ error.line }}, Column {{ error.column }}</div>
            </div>
          </div>
        </div>
      </div>

      <!-- Types Tab -->
      <div v-else-if="activeTab === 'types'" class="output">
        <div style="color: var(--text-muted); padding: 24px; text-align: center;">
          <i class="mdi mdi-shape-outline" style="font-size: 48px; opacity: 0.3;"></i>
          <p style="margin-top: 12px;">Type information coming soon</p>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.count {
  padding: 0 6px;
  font-size: 10px;
  font-weight: 600;
  background: #f85149;
  color: white;
  border-radius: 10px;
}
</style>
