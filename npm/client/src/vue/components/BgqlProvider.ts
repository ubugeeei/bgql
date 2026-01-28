/**
 * BgqlProvider Component
 *
 * Provides BGQL client and query context to child components.
 * Supports both client-only and SSR modes.
 */

import {
  defineComponent,
  provide,
  inject,
  computed,
  h,
  type PropType,
} from 'vue';
import type {
  DocumentNode,
  QueryOptions,
  BgqlQueryContext,
} from '../types';
import type { BgqlClient } from '../../client';
import { useQuery, BGQL_CLIENT_KEY, BGQL_SSR_CONTEXT_KEY } from '../useQuery';
import { BGQL_QUERY_CONTEXT_KEY } from './BgqlDefer';

/**
 * SSR context interface for server-side rendering support.
 */
export interface SSRContext {
  /**
   * Collected data from server-side execution.
   */
  readonly data: Map<string, unknown>;

  /**
   * Pending operations during SSR.
   */
  readonly pending: Set<Promise<unknown>>;

  /**
   * Register data for hydration.
   */
  register(key: string, data: unknown): void;

  /**
   * Get serialized data for client hydration.
   */
  serialize(): string;
}

/**
 * BgqlProvider component that provides client and query context to children.
 *
 * This component can work in two modes:
 * 1. **Client Provider Mode**: Only provides the client instance to children.
 *    Child composables (useQuery, useMutation) will use this client.
 * 2. **Query Provider Mode**: Also executes a query and provides results to children.
 *    Child components (BgqlDefer, BgqlStream) can access query state.
 *
 * @example
 * ```vue
 * <!-- Client Provider Mode - Just provide the client -->
 * <template>
 *   <BgqlProvider :client="client">
 *     <App />
 *   </BgqlProvider>
 * </template>
 *
 * <script setup>
 * import { createClient } from '@bgql/client'
 * import { BgqlProvider } from '@bgql/client/vue'
 *
 * const client = createClient({
 *   url: 'https://api.example.com/graphql',
 * })
 * </script>
 * ```
 *
 * @example
 * ```vue
 * <!-- Query Provider Mode - Execute a query and provide results -->
 * <template>
 *   <BgqlProvider
 *     :query="GetUserQuery"
 *     :variables="{ id: userId }"
 *     streaming
 *     v-slot="{ data, loading, error }"
 *   >
 *     <div v-if="loading && !data">Loading...</div>
 *     <div v-else-if="error">Error: {{ error.message }}</div>
 *     <div v-else>
 *       <h1>{{ data.user.name }}</h1>
 *       <BgqlDefer label="bio">
 *         <template #default>
 *           <p>{{ data.user.bio }}</p>
 *         </template>
 *         <template #fallback>
 *           <Skeleton />
 *         </template>
 *       </BgqlDefer>
 *     </div>
 *   </BgqlProvider>
 * </template>
 * ```
 *
 * @example
 * ```vue
 * <!-- SSR Mode - With server-side rendering context -->
 * <template>
 *   <BgqlProvider :client="client" :ssrContext="ssrContext">
 *     <App />
 *   </BgqlProvider>
 * </template>
 *
 * <script setup>
 * // In your SSR entry file
 * const ssrContext = {
 *   data: new Map(),
 *   pending: new Set(),
 *   register(key, data) {
 *     this.data.set(key, data)
 *   },
 *   serialize() {
 *     return JSON.stringify(Object.fromEntries(this.data))
 *   },
 * }
 * </script>
 * ```
 */
export const BgqlProvider = defineComponent({
  name: 'BgqlProvider',

  props: {
    /**
     * The BGQL client instance.
     * If provided, will be available to all child composables via inject().
     */
    client: {
      type: Object as PropType<BgqlClient>,
      default: undefined,
    },

    /**
     * SSR context for server-side rendering.
     * If provided, enables SSR data collection and hydration.
     */
    ssrContext: {
      type: Object as PropType<SSRContext>,
      default: undefined,
    },

    /**
     * The GraphQL query document or string.
     * If provided, the provider will execute the query and provide results.
     */
    query: {
      type: [Object, String] as PropType<DocumentNode | string>,
      default: undefined,
    },

    /**
     * Variables for the query.
     */
    variables: {
      type: Object as PropType<Record<string, unknown>>,
      default: undefined,
    },

    /**
     * Skip query execution.
     */
    skip: {
      type: Boolean,
      default: false,
    },

    /**
     * Polling interval in milliseconds.
     */
    pollInterval: {
      type: Number,
      default: undefined,
    },

    /**
     * Enable streaming (@defer/@stream) support.
     */
    streaming: {
      type: Boolean,
      default: false,
    },

    /**
     * Fetch policy.
     */
    fetchPolicy: {
      type: String as PropType<QueryOptions['fetchPolicy']>,
      default: 'cache-first',
    },
  },

  emits: ['data', 'error', 'complete'],

  setup(props, { slots, emit }) {
    // Provide client instance if given
    if (props.client) {
      provide(BGQL_CLIENT_KEY, props.client);
    }

    // Provide SSR context if given
    if (props.ssrContext) {
      provide(BGQL_SSR_CONTEXT_KEY, props.ssrContext);
    }

    // If no query is provided, just render children (client-only provider mode)
    if (!props.query) {
      return () => {
        if (slots.default) {
          return slots.default({});
        }
        return null;
      };
    }

    // Query provider mode - execute query and provide results
    const queryResult = useQuery(props.query, {
      variables: () => props.variables,
      skip: () => props.skip,
      pollInterval: props.pollInterval,
      streaming: props.streaming,
      fetchPolicy: props.fetchPolicy,
      client: props.client,
      onData: (data) => emit('data', data),
      onError: (error) => emit('error', error),
      onComplete: () => emit('complete'),
    });

    // Provide query context to child components
    const context = computed<BgqlQueryContext>(() => ({
      data: queryResult.data,
      loading: queryResult.loading,
      error: queryResult.error,
      streamState: queryResult.streamState,
      controller: queryResult.controller,
    }));

    // Use a reactive provide that updates when context changes
    provide(BGQL_QUERY_CONTEXT_KEY, context);

    return () => {
      if (slots.default) {
        return slots.default({
          data: queryResult.data,
          loading: queryResult.loading,
          error: queryResult.error,
          streamState: queryResult.streamState,
          pause: queryResult.pause,
          resume: queryResult.resume,
          refetch: queryResult.refetch,
        });
      }

      return null;
    };
  },
});

/**
 * Create a simple client provider component.
 *
 * This is a convenience function for creating a provider that only
 * provides the client instance without executing a query.
 *
 * @example
 * ```typescript
 * const MyProvider = createClientProvider(myClient)
 *
 * // In template:
 * // <MyProvider>
 * //   <App />
 * // </MyProvider>
 * ```
 */
export function createClientProvider(client: BgqlClient) {
  return defineComponent({
    name: 'BgqlClientProvider',
    setup(_, { slots }) {
      provide(BGQL_CLIENT_KEY, client);
      return () => slots.default?.({});
    },
  });
}

/**
 * Create an SSR context for collecting data during server-side rendering.
 */
export function createSSRContext(): SSRContext {
  const data = new Map<string, unknown>();
  const pending = new Set<Promise<unknown>>();

  return {
    data,
    pending,
    register(key: string, value: unknown) {
      data.set(key, value);
    },
    serialize() {
      return JSON.stringify(Object.fromEntries(data));
    },
  };
}

export default BgqlProvider;
