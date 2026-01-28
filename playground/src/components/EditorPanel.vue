<script setup lang="ts">
import { ref, onMounted, watch, shallowRef } from 'vue'
import * as monaco from 'monaco-editor'

const props = defineProps<{
  modelValue: string
  errors: Array<{ message: string; line: number; column: number }>
}>()

const emit = defineEmits<{
  'update:modelValue': [value: string]
}>()

const editorContainer = ref<HTMLElement>()
const editor = shallowRef<monaco.editor.IStandaloneCodeEditor>()

// Register Better GraphQL language
const registerLanguage = () => {
  monaco.languages.register({ id: 'bgql' })

  monaco.languages.setMonarchTokensProvider('bgql', {
    keywords: [
      'type', 'interface', 'union', 'enum', 'input', 'scalar',
      'schema', 'extend', 'implements', 'fragment', 'on',
      'directive', 'newtype', 'query', 'mutation', 'subscription',
      'Option', 'List', 'extends', 'alias', 'repeatable'
    ],
    typeKeywords: [
      'String', 'Int', 'Uint', 'Float', 'Boolean', 'ID',
      'Date', 'DateTime', 'JSON', 'HTML', 'Void', 'File'
    ],
    operators: ['=', ':', '|', '&', '@', '!'],

    tokenizer: {
      root: [
        [/#.*$/, 'comment'],
        [/"""/, 'string', '@docstring'],
        [/"([^"\\]|\\.)*$/, 'string.invalid'],
        [/"/, 'string', '@string'],
        [/@\w+/, 'annotation'],
        [/[A-Z][\w$]*/, {
          cases: {
            '@typeKeywords': 'type.identifier',
            '@default': 'type'
          }
        }],
        [/[a-z_$][\w$]*/, {
          cases: {
            '@keywords': 'keyword',
            '@default': 'identifier'
          }
        }],
        [/[{}()\[\]]/, '@brackets'],
        [/[<>]/, 'delimiter.angle'],
        [/[,:]/, 'delimiter'],
        [/\d+/, 'number'],
      ],

      string: [
        [/[^\\"]+/, 'string'],
        [/\\./, 'string.escape'],
        [/"/, 'string', '@pop']
      ],

      docstring: [
        [/[^"]+/, 'string.doc'],
        [/"""/, 'string', '@pop'],
        [/"/, 'string.doc']
      ],
    }
  })

  monaco.editor.defineTheme('bgql-dark', {
    base: 'vs-dark',
    inherit: true,
    rules: [
      { token: 'comment', foreground: '6e7681', fontStyle: 'italic' },
      { token: 'keyword', foreground: 'ff7b72' },
      { token: 'type', foreground: 'ffa657' },
      { token: 'type.identifier', foreground: '79c0ff' },
      { token: 'annotation', foreground: 'd2a8ff' },
      { token: 'string', foreground: 'a5d6ff' },
      { token: 'string.doc', foreground: '8b949e' },
      { token: 'number', foreground: '79c0ff' },
      { token: 'identifier', foreground: 'e6edf3' },
      { token: 'delimiter', foreground: 'e6edf3' },
      { token: 'delimiter.angle', foreground: 'ffa657' },
    ],
    colors: {
      'editor.background': '#161b22',
      'editor.foreground': '#e6edf3',
      'editor.lineHighlightBackground': '#21262d',
      'editor.selectionBackground': '#264f78',
      'editorCursor.foreground': '#58a6ff',
      'editorLineNumber.foreground': '#6e7681',
      'editorLineNumber.activeForeground': '#e6edf3',
      'editorIndentGuide.background': '#21262d',
      'editorIndentGuide.activeBackground': '#30363d',
    }
  })
}

onMounted(() => {
  registerLanguage()

  if (editorContainer.value) {
    editor.value = monaco.editor.create(editorContainer.value, {
      value: props.modelValue,
      language: 'bgql',
      theme: 'bgql-dark',
      fontSize: 13,
      fontFamily: "'JetBrains Mono', 'Fira Code', monospace",
      fontLigatures: true,
      lineNumbers: 'on',
      minimap: { enabled: false },
      scrollBeyondLastLine: false,
      automaticLayout: true,
      tabSize: 2,
      insertSpaces: true,
      padding: { top: 12 },
      renderLineHighlight: 'line',
      smoothScrolling: true,
      cursorBlinking: 'smooth',
      cursorSmoothCaretAnimation: 'on',
    })

    editor.value.onDidChangeModelContent(() => {
      emit('update:modelValue', editor.value?.getValue() ?? '')
    })
  }
})

watch(() => props.errors, (newErrors) => {
  if (!editor.value) return

  const model = editor.value.getModel()
  if (!model) return

  const markers = newErrors.map(err => ({
    severity: monaco.MarkerSeverity.Error,
    message: err.message,
    startLineNumber: err.line,
    startColumn: err.column,
    endLineNumber: err.line,
    endColumn: err.column + 1,
  }))

  monaco.editor.setModelMarkers(model, 'bgql', markers)
})
</script>

<template>
  <div class="panel">
    <div class="panel__header">
      <div class="title">
        <i class="mdi mdi-code-braces"></i>
        Schema Editor
      </div>
      <div class="actions">
        <button class="btn btn-ghost btn-icon" title="Format">
          <i class="mdi mdi-format-align-left"></i>
        </button>
        <button class="btn btn-ghost btn-icon" title="Copy">
          <i class="mdi mdi-content-copy"></i>
        </button>
      </div>
    </div>
    <div class="panel__content" ref="editorContainer"></div>
  </div>
</template>
