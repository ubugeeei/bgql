/**
 * Vue SDK for BGQL
 *
 * Framework-agnostic component model with streaming support.
 */

// Types
export type {
  DocumentNode,
  QueryOptions,
  FetchPolicy,
  StreamState,
  ExecutionController,
  Checkpoint,
  ExecutionPosition,
  BinaryStreamHandle,
  BinaryStreamState,
  BufferedRange,
  ServerFragmentOptions,
  CacheStrategy,
  HydrationStrategy,
  HydrationPriority,
  MultipartChunk,
  BgqlQueryContext,
  UseQueryResult,
  UseServerFragmentResult,
  UseBinaryStreamResult,
} from './types';

// Composables
export { useQuery } from './useQuery';
export {
  useServerFragment,
  useServerFragmentAsync,
  clearServerCache,
  clearRequestCache,
  getSSRState,
} from './useServerFragment';
export {
  useBinaryStream,
  setupMediaSource,
  createBlobUrl,
  downloadBinaryStream,
} from './useBinaryStream';

// Components
export {
  // Barrel export (recommended)
  Bgql,
  // Legacy named exports
  BgqlDefer,
  BgqlStream,
  BgqlBinaryStream,
  BgqlProvider,
  // Utilities
  createTypedDefer,
  createTypedStream,
  BGQL_QUERY_CONTEXT_KEY,
} from './components';

// Vite Plugin
export {
  bgqlPlugin,
  createSSRContext,
  serializeSSRContext,
  generateHydrationScript,
  type BgqlPluginOptions as VitePluginOptions,
  type BgqlSSRContext,
} from './vite-plugin';

// =============================================================================
// Plugin
// =============================================================================

import type { App } from 'vue';
import { BgqlDefer, BgqlStream, BgqlBinaryStream, BgqlProvider } from './components';

/**
 * BGQL Vue plugin options.
 */
export interface BgqlPluginOptions {
  /**
   * GraphQL endpoint URL.
   */
  endpoint?: string;

  /**
   * Default headers for all requests.
   */
  headers?: Record<string, string>;

  /**
   * Component name prefix.
   * @default 'Bgql'
   */
  componentPrefix?: string;
}

/**
 * BGQL Vue plugin.
 *
 * @example
 * ```typescript
 * import { createApp } from 'vue'
 * import { BgqlPlugin } from '@bgql/client/vue'
 *
 * const app = createApp(App)
 *
 * app.use(BgqlPlugin, {
 *   endpoint: 'https://api.example.com/graphql',
 *   headers: {
 *     'Authorization': `Bearer ${token}`,
 *   },
 * })
 *
 * app.mount('#app')
 * ```
 */
export const BgqlPlugin = {
  install(app: App, options: BgqlPluginOptions = {}): void {
    // Set global endpoint
    if (options.endpoint) {
      if (typeof window !== 'undefined') {
        (window as unknown as { __BGQL_ENDPOINT__: string }).__BGQL_ENDPOINT__ =
          options.endpoint;
      }
    }

    // Set global headers
    if (options.headers) {
      if (typeof window !== 'undefined') {
        (window as unknown as { __BGQL_HEADERS__: Record<string, string> }).__BGQL_HEADERS__ =
          options.headers;
      }
    }

    // Register components
    const prefix = options.componentPrefix ?? 'Bgql';

    app.component(`${prefix}Defer`, BgqlDefer);
    app.component(`${prefix}Stream`, BgqlStream);
    app.component(`${prefix}BinaryStream`, BgqlBinaryStream);
    app.component(`${prefix}Provider`, BgqlProvider);
  },
};

export default BgqlPlugin;
