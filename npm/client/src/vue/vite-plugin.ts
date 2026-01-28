/**
 * BGQL Vite Plugin
 *
 * Framework-agnostic SSR support for BGQL Server Fragments.
 * Uses Vite Environment API for proper SSR/Client separation.
 */

import type { Plugin, PluginOption, ResolvedConfig } from 'vite'

export interface BgqlPluginOptions {
  /**
   * GraphQL endpoint URL
   * @default '/graphql'
   */
  endpoint?: string

  /**
   * WebSocket endpoint for subscriptions
   * @default '/graphql/ws'
   */
  wsEndpoint?: string

  /**
   * Binary stream endpoint
   * @default '/graphql/binary'
   */
  binaryEndpoint?: string

  /**
   * Enable Server Fragments SSR
   * @default true
   */
  ssr?: boolean

  /**
   * Server Fragment cache strategy
   * @default 'request'
   */
  cacheStrategy?: 'none' | 'request' | 'user' | 'global'

  /**
   * Development mode features
   */
  dev?: {
    /**
     * Enable GraphQL Playground
     * @default true
     */
    playground?: boolean

    /**
     * Enable query logging
     * @default false
     */
    logging?: boolean
  }
}

/**
 * Server Fragment data collected during SSR
 */
interface ServerFragmentData {
  id: string
  fragmentName: string
  data: unknown
  timestamp: number
}

/**
 * SSR context for collecting server fragment data
 */
export interface BgqlSSRContext {
  fragments: Map<string, ServerFragmentData>
  pendingDefers: Map<string, Promise<unknown>>
  streamStates: Map<string, { cursor: string | null; hasNext: boolean }>
}

/**
 * Create SSR context for a request
 */
export function createSSRContext(): BgqlSSRContext {
  return {
    fragments: new Map(),
    pendingDefers: new Map(),
    streamStates: new Map(),
  }
}

/**
 * Serialize SSR context for client hydration
 */
export function serializeSSRContext(ctx: BgqlSSRContext): string {
  const data = {
    fragments: Object.fromEntries(ctx.fragments),
    streamStates: Object.fromEntries(ctx.streamStates),
  }
  return JSON.stringify(data)
}

/**
 * Generate hydration script to inject into HTML
 */
export function generateHydrationScript(ctx: BgqlSSRContext): string {
  const serialized = serializeSSRContext(ctx)
  return `<script>window.__BGQL_SSR_DATA__=${serialized}</script>`
}

/**
 * Virtual module ID for BGQL config
 */
const VIRTUAL_MODULE_ID = 'virtual:bgql-config'
const RESOLVED_VIRTUAL_MODULE_ID = '\0' + VIRTUAL_MODULE_ID

/**
 * BGQL Vite Plugin using Environment API
 */
export function bgqlPlugin(options: BgqlPluginOptions = {}): PluginOption {
  const {
    endpoint = '/graphql',
    wsEndpoint = '/graphql/ws',
    binaryEndpoint = '/graphql/binary',
    ssr = true,
    cacheStrategy = 'request',
    dev = {},
  } = options

  const { playground = true, logging = false } = dev

  let resolvedConfig: ResolvedConfig

  // Base plugin for both client and server environments
  const basePlugin: Plugin = {
    name: 'vite-plugin-bgql',

    configResolved(config) {
      resolvedConfig = config
      if (logging) {
        console.log('[BGQL] Plugin configured:', {
          endpoint,
          wsEndpoint,
          binaryEndpoint,
          ssr,
          cacheStrategy,
          mode: config.mode,
        })
      }
    },

    resolveId(id) {
      if (id === VIRTUAL_MODULE_ID) {
        return RESOLVED_VIRTUAL_MODULE_ID
      }
      return null
    },
  }

  // Client-specific plugin
  const clientPlugin: Plugin = {
    name: 'vite-plugin-bgql:client',
    apply: 'build',

    config(config) {
      // Ensure client build configuration
      return {
        ...config,
        define: {
          ...config.define,
          __BGQL_ENDPOINT__: JSON.stringify(endpoint),
          __BGQL_WS_ENDPOINT__: JSON.stringify(wsEndpoint),
          __BGQL_BINARY_ENDPOINT__: JSON.stringify(binaryEndpoint),
        },
      }
    },

    load(id) {
      if (id === RESOLVED_VIRTUAL_MODULE_ID) {
        // Client-side module: reads from window or uses defaults
        return `
export const endpoint = ${JSON.stringify(endpoint)}
export const wsEndpoint = ${JSON.stringify(wsEndpoint)}
export const binaryEndpoint = ${JSON.stringify(binaryEndpoint)}
export const ssrEnabled = ${ssr}
export const cacheStrategy = ${JSON.stringify(cacheStrategy)}

export function getConfig() {
  if (typeof window !== 'undefined' && window.__BGQL_CONFIG__) {
    return window.__BGQL_CONFIG__
  }
  return { endpoint, wsEndpoint, binaryEndpoint }
}

export function getSSRData() {
  if (typeof window !== 'undefined' && window.__BGQL_SSR_DATA__) {
    return window.__BGQL_SSR_DATA__
  }
  return null
}

export function hydrateFromSSR() {
  const ssrData = getSSRData()
  if (!ssrData) return null

  return {
    fragments: new Map(Object.entries(ssrData.fragments || {})),
    streamStates: new Map(Object.entries(ssrData.streamStates || {})),
  }
}
`
      }
      return null
    },

    transformIndexHtml: {
      order: 'post',
      handler(html) {
        // Inject BGQL client configuration
        const config = { endpoint, wsEndpoint, binaryEndpoint }
        const configScript = `<script>window.__BGQL_CONFIG__=${JSON.stringify(config)}</script>`
        return html.replace('</head>', `${configScript}</head>`)
      },
    },
  }

  // Server-specific plugin (SSR environment)
  const serverPlugin: Plugin = {
    name: 'vite-plugin-bgql:server',

    // Apply only during SSR build
    apply(config, { isSsrBuild }) {
      return isSsrBuild === true
    },

    load(id) {
      if (id === RESOLVED_VIRTUAL_MODULE_ID) {
        // Server-side module: provides utilities for SSR rendering
        return `
export const endpoint = ${JSON.stringify(endpoint)}
export const wsEndpoint = ${JSON.stringify(wsEndpoint)}
export const binaryEndpoint = ${JSON.stringify(binaryEndpoint)}
export const ssrEnabled = ${ssr}
export const cacheStrategy = ${JSON.stringify(cacheStrategy)}
export const isServer = true

// Server-side context management
const requestContexts = new WeakMap()

export function createRequestContext(req) {
  const ctx = {
    fragments: new Map(),
    pendingDefers: new Map(),
    streamStates: new Map(),
  }
  if (req) {
    requestContexts.set(req, ctx)
  }
  return ctx
}

export function getRequestContext(req) {
  return requestContexts.get(req)
}

export function serializeContext(ctx) {
  return JSON.stringify({
    fragments: Object.fromEntries(ctx.fragments),
    streamStates: Object.fromEntries(ctx.streamStates),
  })
}

export function generateHydrationScript(ctx) {
  const serialized = serializeContext(ctx)
  return '<script>window.__BGQL_SSR_DATA__=' + serialized + '</script>'
}

// Server fragment execution helper
export async function executeServerFragment(fragmentName, variables, ctx) {
  // This would be implemented by the user's GraphQL client
  throw new Error('executeServerFragment must be implemented by the application')
}
`
      }
      return null
    },
  }

  // Development server plugin
  const devPlugin: Plugin = {
    name: 'vite-plugin-bgql:dev',
    apply: 'serve',

    configureServer(server) {
      // Add GraphQL Playground in development
      if (playground) {
        server.middlewares.use('/__bgql_playground', (_req, res) => {
          res.setHeader('Content-Type', 'text/html')
          res.end(generatePlaygroundHTML(endpoint, wsEndpoint))
        })

        if (logging) {
          console.log('[BGQL] GraphQL Playground: http://localhost:' + (resolvedConfig?.server?.port || 5173) + '/__bgql_playground')
        }
      }

      // SSR context middleware
      if (ssr) {
        server.middlewares.use((req, _res, next) => {
          // Attach SSR context to request
          const reqWithContext = req as typeof req & { bgqlSSRContext?: BgqlSSRContext }
          reqWithContext.bgqlSSRContext = createSSRContext()
          next()
        })
      }
    },

    load(id) {
      if (id === RESOLVED_VIRTUAL_MODULE_ID) {
        // Development mode: combined client/server module
        return `
export const endpoint = ${JSON.stringify(endpoint)}
export const wsEndpoint = ${JSON.stringify(wsEndpoint)}
export const binaryEndpoint = ${JSON.stringify(binaryEndpoint)}
export const ssrEnabled = ${ssr}
export const cacheStrategy = ${JSON.stringify(cacheStrategy)}
export const isDev = true

export function getConfig() {
  if (typeof window !== 'undefined' && window.__BGQL_CONFIG__) {
    return window.__BGQL_CONFIG__
  }
  return { endpoint, wsEndpoint, binaryEndpoint }
}

export function getSSRData() {
  if (typeof window !== 'undefined' && window.__BGQL_SSR_DATA__) {
    return window.__BGQL_SSR_DATA__
  }
  return null
}

export function hydrateFromSSR() {
  const ssrData = getSSRData()
  if (!ssrData) return null
  return {
    fragments: new Map(Object.entries(ssrData.fragments || {})),
    streamStates: new Map(Object.entries(ssrData.streamStates || {})),
  }
}
`
      }
      return null
    },

    transformIndexHtml: {
      order: 'post',
      handler(html, ctx) {
        const config = { endpoint, wsEndpoint, binaryEndpoint }
        const configScript = `<script>window.__BGQL_CONFIG__=${JSON.stringify(config)}</script>`

        // Inject SSR data if available
        const ssrContext = (ctx as typeof ctx & { bgqlSSRContext?: BgqlSSRContext }).bgqlSSRContext
        const ssrScript = ssrContext ? generateHydrationScript(ssrContext) : ''

        return html.replace('</head>', `${configScript}${ssrScript}</head>`)
      },
    },
  }

  return [basePlugin, clientPlugin, serverPlugin, devPlugin]
}

/**
 * Generate GraphQL Playground HTML
 */
function generatePlaygroundHTML(endpoint: string, wsEndpoint: string): string {
  return `<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>BGQL Playground</title>
  <link rel="stylesheet" href="https://unpkg.com/graphiql/graphiql.min.css" />
</head>
<body style="margin: 0; overflow: hidden;">
  <div id="graphiql" style="height: 100vh;"></div>
  <script crossorigin src="https://unpkg.com/react/umd/react.production.min.js"></script>
  <script crossorigin src="https://unpkg.com/react-dom/umd/react-dom.production.min.js"></script>
  <script crossorigin src="https://unpkg.com/graphiql/graphiql.min.js"></script>
  <script>
    const fetcher = GraphiQL.createFetcher({
      url: '${endpoint}',
      subscriptionUrl: location.protocol === 'https:'
        ? 'wss://' + location.host + '${wsEndpoint}'
        : 'ws://' + location.host + '${wsEndpoint}',
    });
    ReactDOM.render(
      React.createElement(GraphiQL, { fetcher }),
      document.getElementById('graphiql'),
    );
  </script>
</body>
</html>`
}

// Type augmentation for global window
declare global {
  interface Window {
    __BGQL_CONFIG__?: {
      endpoint: string
      wsEndpoint: string
      binaryEndpoint: string
    }
    __BGQL_SSR_DATA__?: {
      fragments: Record<string, ServerFragmentData>
      streamStates: Record<string, { cursor: string | null; hasNext: boolean }>
    }
  }
}

export default bgqlPlugin
