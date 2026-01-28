/**
 * useServerFragment Composable for BGQL
 *
 * Vue composable for server-side fragments with @server directive support.
 */

import {
  ref,
  onMounted,
  onServerPrefetch,
  type Ref,
} from 'vue';
import type {
  ServerFragmentOptions,
  UseServerFragmentResult,
  CacheStrategy,
} from './types';

/**
 * Server-side fragment cache.
 */
const serverCache = new Map<string, { data: unknown; timestamp: number }>();

/**
 * Cache TTL by strategy (in milliseconds).
 */
const CACHE_TTL: Record<CacheStrategy, number> = {
  none: 0,
  request: 0, // Cleared per request
  user: 1000 * 60 * 60, // 1 hour
  global: 1000 * 60 * 60 * 24, // 24 hours
};

/**
 * Fetches and caches server fragment data.
 *
 * This composable is designed to work with @server fragments that are
 * only executed on the server. It supports:
 * - Server-side rendering with onServerPrefetch
 * - Caching strategies (none, request, user, global)
 * - Automatic hydration on client
 *
 * @example
 * ```vue
 * <script setup>
 * import { useServerFragment } from '@bgql/client/vue'
 *
 * // Define the fragment in your schema:
 * // fragment UserProfile on User @server {
 * //   id
 * //   name
 * //   email
 * // }
 *
 * const { data, loading, error } = await useServerFragment<UserProfile>({
 *   fragmentName: 'UserProfile',
 *   variables: { userId: props.userId },
 *   cache: 'user',
 * })
 * </script>
 * ```
 */
export function useServerFragment<TData = unknown>(
  options: ServerFragmentOptions
): UseServerFragmentResult<TData> {
  const data = ref<TData | null>(null) as Ref<TData | null>;
  const loading = ref(true);
  const error = ref<Error | null>(null);

  const cacheKey = getCacheKey(options);
  const cacheTtl = CACHE_TTL[options.cache ?? 'none'];

  const fetchFragment = async (): Promise<void> => {
    // Check cache first
    if (cacheTtl > 0) {
      const cached = serverCache.get(cacheKey);
      if (cached && Date.now() - cached.timestamp < cacheTtl) {
        data.value = cached.data as TData;
        loading.value = false;
        return;
      }
    }

    try {
      const response = await fetch(getBgqlEndpoint(), {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          Accept: 'application/json',
          'X-BGQL-Server-Fragment': options.fragmentName,
        },
        body: JSON.stringify({
          query: buildFragmentQuery(options.fragmentName),
          variables: options.variables,
        }),
      });

      if (!response.ok) {
        throw new Error(`HTTP ${response.status}: ${response.statusText}`);
      }

      const result = await response.json();

      if (result.errors?.length) {
        throw new Error(result.errors[0].message);
      }

      data.value = result.data;

      // Cache the result
      if (cacheTtl > 0) {
        serverCache.set(cacheKey, {
          data: result.data,
          timestamp: Date.now(),
        });
      }
    } catch (err) {
      error.value = err instanceof Error ? err : new Error(String(err));
    } finally {
      loading.value = false;
    }
  };

  // Server-side prefetch for SSR
  onServerPrefetch(async () => {
    await fetchFragment();
  });

  // Client-side hydration
  onMounted(async () => {
    // Check if we have SSR data
    const ssrData = getSSRData<TData>(cacheKey);
    if (ssrData !== undefined) {
      data.value = ssrData;
      loading.value = false;
      return;
    }

    // Fetch if not available
    if (data.value === null) {
      await fetchFragment();
    }
  });

  return {
    get data() {
      return data.value;
    },
    get loading() {
      return loading.value;
    },
    get error() {
      return error.value;
    },
  };
}

/**
 * Async version for use with top-level await.
 */
export async function useServerFragmentAsync<TData = unknown>(
  options: ServerFragmentOptions
): Promise<UseServerFragmentResult<TData>> {
  const data = ref<TData | null>(null) as Ref<TData | null>;
  const loading = ref(true);
  const error = ref<Error | null>(null);

  const cacheKey = getCacheKey(options);
  const cacheTtl = CACHE_TTL[options.cache ?? 'none'];

  // Check cache first
  if (cacheTtl > 0) {
    const cached = serverCache.get(cacheKey);
    if (cached && Date.now() - cached.timestamp < cacheTtl) {
      data.value = cached.data as TData;
      loading.value = false;
      return {
        get data() {
          return data.value;
        },
        get loading() {
          return loading.value;
        },
        get error() {
          return error.value;
        },
      };
    }
  }

  try {
    const response = await fetch(getBgqlEndpoint(), {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        Accept: 'application/json',
        'X-BGQL-Server-Fragment': options.fragmentName,
      },
      body: JSON.stringify({
        query: buildFragmentQuery(options.fragmentName),
        variables: options.variables,
      }),
    });

    if (!response.ok) {
      throw new Error(`HTTP ${response.status}: ${response.statusText}`);
    }

    const result = await response.json();

    if (result.errors?.length) {
      throw new Error(result.errors[0].message);
    }

    data.value = result.data;

    // Cache the result
    if (cacheTtl > 0) {
      serverCache.set(cacheKey, {
        data: result.data,
        timestamp: Date.now(),
      });
    }
  } catch (err) {
    error.value = err instanceof Error ? err : new Error(String(err));
  } finally {
    loading.value = false;
  }

  return {
    get data() {
      return data.value;
    },
    get loading() {
      return loading.value;
    },
    get error() {
      return error.value;
    },
  };
}

// =============================================================================
// Helper Functions
// =============================================================================

function getBgqlEndpoint(): string {
  if (typeof window !== 'undefined' && (window as unknown as { __BGQL_ENDPOINT__?: string }).__BGQL_ENDPOINT__) {
    return (window as unknown as { __BGQL_ENDPOINT__?: string }).__BGQL_ENDPOINT__!;
  }
  return '/graphql';
}

function getCacheKey(options: ServerFragmentOptions): string {
  return `${options.fragmentName}:${JSON.stringify(options.variables ?? {})}`;
}

function buildFragmentQuery(fragmentName: string): string {
  // The server knows how to expand this based on the fragment definition
  return `query __ServerFragment__ { __fragment(name: "${fragmentName}") }`;
}

function getSSRData<T>(key: string): T | undefined {
  if (typeof window === 'undefined') {
    return undefined;
  }

  const ssrState = (window as unknown as { __BGQL_SSR_STATE__?: Record<string, unknown> }).__BGQL_SSR_STATE__;
  if (ssrState && key in ssrState) {
    return ssrState[key] as T;
  }

  return undefined;
}

/**
 * Clears the server fragment cache.
 * Call this at the start of each request in SSR.
 */
export function clearServerCache(): void {
  serverCache.clear();
}

/**
 * Clears request-scoped cache entries.
 * Call this at the end of each request in SSR.
 */
export function clearRequestCache(): void {
  // Request cache is always cleared, nothing stored with 'request' strategy
}

/**
 * Gets all cached data for SSR hydration.
 */
export function getSSRState(): Record<string, unknown> {
  const state: Record<string, unknown> = {};
  for (const [key, value] of serverCache) {
    state[key] = value.data;
  }
  return state;
}
