import { ref, shallowRef } from 'vue'
import init, { BetterGraphQL, version as wasmVersion } from 'bgql_wasm'

// Types matching the WASM module output
export interface Diagnostic {
  severity: 'error' | 'warning'
  message: string
  code: string
  start_line: number
  start_column: number
  end_line: number
  end_column: number
}

export interface FieldInfo {
  name: string
  type_name: string
  description?: string
  arguments: ArgumentInfo[]
  directives: DirectiveInfo[]
}

export interface ArgumentInfo {
  name: string
  type_name: string
  default_value?: string
}

export interface DirectiveInfo {
  name: string
  arguments: [string, string][]
}

export interface TypeInfo {
  name: string
  kind: string
  description?: string
  fields: FieldInfo[]
  implements: string[]
  values: string[]
  members: string[]
}

export interface SchemaDefinition {
  query_type?: string
  mutation_type?: string
  subscription_type?: string
  directives: DirectiveInfo[]
}

export interface FragmentInfo {
  name: string
  on_type: string
}

export interface ParseResult {
  success: boolean
  diagnostics: Diagnostic[]
  types: TypeInfo[]
  schema?: SchemaDefinition
  fragments: FragmentInfo[]
}

export function useBetterGraphQL() {
  const isLoading = ref(true)
  const isReady = ref(false)
  const error = ref<string | null>(null)
  const bgqlInstance = shallowRef<BetterGraphQL | null>(null)
  const version = ref<string>('loading...')

  // Initialize WASM module
  const initWasm = async () => {
    isLoading.value = true
    error.value = null

    try {
      // Initialize WASM module
      await init()

      // Create instance
      bgqlInstance.value = new BetterGraphQL()
      version.value = wasmVersion()
      isReady.value = true
    } catch (e) {
      error.value = String(e)
      console.error('Failed to initialize WASM module:', e)
      version.value = 'error'
    } finally {
      isLoading.value = false
    }
  }

  const parse = (source: string): ParseResult => {
    if (!bgqlInstance.value) {
      return {
        success: false,
        diagnostics: [{
          severity: 'error',
          message: 'WASM module not initialized',
          code: 'WASM_NOT_READY',
          start_line: 1,
          start_column: 1,
          end_line: 1,
          end_column: 1,
        }],
        types: [],
        fragments: [],
      }
    }

    try {
      return bgqlInstance.value.parse(source) as ParseResult
    } catch (e) {
      return {
        success: false,
        diagnostics: [{
          severity: 'error',
          message: String(e),
          code: 'PARSE_ERROR',
          start_line: 1,
          start_column: 1,
          end_line: 1,
          end_column: 1,
        }],
        types: [],
        fragments: [],
      }
    }
  }

  const validate = (source: string): { valid: boolean; diagnostics: Diagnostic[] } => {
    if (!bgqlInstance.value) {
      return {
        valid: false,
        diagnostics: [{
          severity: 'error',
          message: 'WASM module not initialized',
          code: 'WASM_NOT_READY',
          start_line: 1,
          start_column: 1,
          end_line: 1,
          end_column: 1,
        }],
      }
    }

    try {
      return bgqlInstance.value.validate(source) as { valid: boolean; diagnostics: Diagnostic[] }
    } catch (e) {
      return {
        valid: false,
        diagnostics: [{
          severity: 'error',
          message: String(e),
          code: 'VALIDATE_ERROR',
          start_line: 1,
          start_column: 1,
          end_line: 1,
          end_column: 1,
        }],
      }
    }
  }

  const format = (source: string): string => {
    if (!bgqlInstance.value) {
      return source
    }

    try {
      return bgqlInstance.value.format(source)
    } catch (e) {
      console.error('Format error:', e)
      return source
    }
  }

  // Initialize on first use
  initWasm()

  return {
    isLoading,
    isReady,
    error,
    version,
    parse,
    validate,
    format,
  }
}
