<script setup lang="ts">
import { ref, watch, watchEffect } from 'vue'
import Header from './components/Header.vue'
import EditorPanel from './components/EditorPanel.vue'
import OutputPanel from './components/OutputPanel.vue'
import StatusBar from './components/StatusBar.vue'
import { useBetterGraphQL, type ParseResult } from './composables/useBetterGraphQL'

const { parse, isLoading: wasmLoading, isReady: wasmReady, version } = useBetterGraphQL()

const schema = ref(`# bgql Schema Example

"""User account in the system"""
type User implements Node {
  id: UserId
  email: String @email
  name: String
  avatarUrl: Option<String>
  posts: List<Post>
  createdAt: DateTime
}

"""Blog post"""
type Post implements Node {
  id: PostId
  title: String @minLength(1) @maxLength(200)
  content: HTML
  author: User
  tags: List<Tag>
  publishedAt: Option<DateTime>
}

type Tag implements Node {
  id: TagId
  name: String
  slug: String
}

# Opaque type definitions for type-safe IDs
opaque UserId = ID
opaque PostId = ID
opaque TagId = ID

# Query type
type Query {
  me @requireAuth: Option<User>
  user(id: UserId): UserResult
  posts(first: Int = 10, after: Option<String>): PostConnection
}

# Result type using Rust-style enum with data
enum UserResult {
  Ok(User)
  NotFound { id: UserId }
  Unauthorized { message: String }
}

# Connection type with generics
type Connection<T extends Node> {
  edges: List<Edge<T>>
  pageInfo: PageInfo
  totalCount: Uint
}

type Edge<T> {
  cursor: String
  node: T
}

type alias PostConnection = Connection<Post>

interface Node {
  id: ID
}
`)

const output = ref<string>('')
const parseResult = ref<ParseResult | null>(null)
const errors = ref<Array<{message: string, line: number, column: number}>>([])
const isProcessing = ref(false)
const leftPanelWidth = ref(50)

// Debounce timer
let debounceTimer: ReturnType<typeof setTimeout> | null = null

const parseSchema = () => {
  if (!wasmReady.value) {
    return
  }

  isProcessing.value = true

  try {
    const result = parse(schema.value)
    parseResult.value = result

    // Convert diagnostics to error format for editor
    errors.value = result.diagnostics
      .filter(d => d.severity === 'error')
      .map(d => ({
        message: d.message,
        line: d.start_line,
        column: d.start_column,
      }))

    // Format output
    output.value = JSON.stringify({
      success: result.success,
      types: result.types.map(t => ({
        name: t.name,
        kind: t.kind,
        description: t.description,
        fields: t.fields.length > 0 ? t.fields : undefined,
        implements: t.implements.length > 0 ? t.implements : undefined,
        values: t.values.length > 0 ? t.values : undefined,
        members: t.members.length > 0 ? t.members : undefined,
      })),
      schema: result.schema,
      fragments: result.fragments.length > 0 ? result.fragments : undefined,
      diagnostics: result.diagnostics.length > 0 ? result.diagnostics : undefined,
    }, null, 2)
  } catch (e) {
    errors.value = [{
      message: String(e),
      line: 1,
      column: 1,
    }]
    output.value = JSON.stringify({ error: String(e) }, null, 2)
  }

  isProcessing.value = false
}

const debouncedParse = () => {
  if (debounceTimer) {
    clearTimeout(debounceTimer)
  }
  debounceTimer = setTimeout(() => {
    parseSchema()
  }, 300)
}

const handleResize = (e: MouseEvent) => {
  const startX = e.clientX
  const startWidth = leftPanelWidth.value

  const onMouseMove = (e: MouseEvent) => {
    const delta = e.clientX - startX
    const containerWidth = window.innerWidth
    leftPanelWidth.value = Math.max(20, Math.min(80, startWidth + (delta / containerWidth) * 100))
  }

  const onMouseUp = () => {
    document.removeEventListener('mousemove', onMouseMove)
    document.removeEventListener('mouseup', onMouseUp)
  }

  document.addEventListener('mousemove', onMouseMove)
  document.addEventListener('mouseup', onMouseUp)
}

// Watch for WASM ready and parse when ready
watchEffect(() => {
  if (wasmReady.value) {
    parseSchema()
  }
})

// Watch for schema changes
watch(schema, debouncedParse)
</script>

<template>
  <Header />

  <main class="main">
    <EditorPanel
      v-model="schema"
      :style="{ width: leftPanelWidth + '%' }"
      :errors="errors"
    />

    <div class="resizer" @mousedown="handleResize"></div>

    <OutputPanel
      :style="{ width: (100 - leftPanelWidth) + '%' }"
      :output="output"
      :errors="errors"
      :is-processing="isProcessing || wasmLoading"
    />
  </main>

  <StatusBar
    :errors="errors.length"
    :is-processing="isProcessing || wasmLoading"
    :version="version"
  />
</template>
