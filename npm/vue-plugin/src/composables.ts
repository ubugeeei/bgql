/**
 * Vue composables for Better GraphQL.
 *
 * @example
 * ```vue
 * <script lang="gql">
 * query GetUser($id: ID!) {
 *   user(id: $id) { id name }
 * }
 * </script>
 *
 * <script setup lang="ts">
 * import { GetUser } from './Component.vue?gql';
 * import { useQuery } from '@bgql/vue-plugin';
 *
 * const { data, loading, error, refetch } = useQuery(GetUser, { id: '1' });
 * </script>
 *
 * <template>
 *   <div v-if="loading">Loading...</div>
 *   <div v-else-if="error">{{ error.message }}</div>
 *   <div v-else>{{ data?.user?.name }}</div>
 * </template>
 * ```
 */

import { ref, shallowRef, computed, watch, type Ref } from "vue";
import {
  type TypedDocumentNode,
  type BgqlClient,
  type SdkError,
  type Result,
  isOk,
} from "@bgql/sdk";

/**
 * Query state.
 */
export interface QueryState<TData> {
  data: Ref<TData | null>;
  loading: Ref<boolean>;
  error: Ref<SdkError | null>;
  called: Ref<boolean>;
}

/**
 * Query result with refetch capability.
 */
export interface UseQueryResult<TData, TVariables> extends QueryState<TData> {
  refetch: (variables?: TVariables) => Promise<void>;
}

/**
 * Mutation result.
 */
export interface UseMutationResult<TData, TVariables> extends QueryState<TData> {
  mutate: (variables: TVariables) => Promise<Result<TData>>;
  reset: () => void;
}

// Client injection key
const CLIENT_KEY = Symbol("bgql-client");

/**
 * Create a client provider.
 */
export function provideClient(client: BgqlClient): void {
  // In Vue 3, this would use provide/inject
  (globalThis as Record<symbol, unknown>)[CLIENT_KEY] = client;
}

/**
 * Get the injected client.
 */
export function useClient(): BgqlClient {
  const client = (globalThis as Record<symbol, unknown>)[CLIENT_KEY] as BgqlClient | undefined;
  if (!client) {
    throw new Error(
      "BgqlClient not provided. Call provideClient(client) in your app setup."
    );
  }
  return client;
}

/**
 * Execute a GraphQL query with reactive state.
 */
export function useQuery<TVariables extends Record<string, unknown>, TData>(
  operation: TypedDocumentNode<TVariables, TData>,
  variables: TVariables | Ref<TVariables>,
  options?: {
    client?: BgqlClient;
    skip?: Ref<boolean> | boolean;
    onCompleted?: (data: TData) => void;
    onError?: (error: SdkError) => void;
  }
): UseQueryResult<TData, TVariables> {
  const client = options?.client ?? useClient();

  const data = shallowRef<TData | null>(null);
  const loading = ref(false);
  const error = shallowRef<SdkError | null>(null);
  const called = ref(false);

  const varsRef = ref(variables) as Ref<TVariables>;

  async function execute(vars?: TVariables): Promise<void> {
    const currentVars = vars ?? varsRef.value;

    loading.value = true;
    error.value = null;
    called.value = true;

    try {
      const result = await client.execute(operation, currentVars);

      if (isOk(result)) {
        data.value = result.value;
        options?.onCompleted?.(result.value);
      } else {
        error.value = result.error;
        options?.onError?.(result.error);
      }
    } finally {
      loading.value = false;
    }
  }

  // Auto-execute on mount and variable changes
  const skipRef = computed(() =>
    typeof options?.skip === "boolean" ? options.skip : options?.skip?.value ?? false
  );

  if (!skipRef.value) {
    execute();
  }

  watch(
    [varsRef, skipRef],
    () => {
      if (!skipRef.value) {
        execute();
      }
    },
    { deep: true }
  );

  return {
    data,
    loading,
    error,
    called,
    refetch: execute,
  };
}

/**
 * Execute a GraphQL mutation.
 */
export function useMutation<TVariables extends Record<string, unknown>, TData>(
  operation: TypedDocumentNode<TVariables, TData>,
  options?: {
    client?: BgqlClient;
    onCompleted?: (data: TData) => void;
    onError?: (error: SdkError) => void;
  }
): UseMutationResult<TData, TVariables> {
  const client = options?.client ?? useClient();

  const data = shallowRef<TData | null>(null);
  const loading = ref(false);
  const error = shallowRef<SdkError | null>(null);
  const called = ref(false);

  async function mutate(variables: TVariables): Promise<Result<TData>> {
    loading.value = true;
    error.value = null;
    called.value = true;

    try {
      const result = await client.execute(operation, variables);

      if (isOk(result)) {
        data.value = result.value;
        options?.onCompleted?.(result.value);
      } else {
        error.value = result.error;
        options?.onError?.(result.error);
      }

      return result;
    } finally {
      loading.value = false;
    }
  }

  function reset(): void {
    data.value = null;
    loading.value = false;
    error.value = null;
    called.value = false;
  }

  return {
    data,
    loading,
    error,
    called,
    mutate,
    reset,
  };
}

/**
 * Lazy query that doesn't execute until called.
 */
export function useLazyQuery<TVariables extends Record<string, unknown>, TData>(
  operation: TypedDocumentNode<TVariables, TData>,
  options?: {
    client?: BgqlClient;
    onCompleted?: (data: TData) => void;
    onError?: (error: SdkError) => void;
  }
): [
  (variables: TVariables) => Promise<Result<TData>>,
  QueryState<TData> & { called: Ref<boolean> }
] {
  const client = options?.client ?? useClient();

  const data = shallowRef<TData | null>(null);
  const loading = ref(false);
  const error = shallowRef<SdkError | null>(null);
  const called = ref(false);

  async function execute(variables: TVariables): Promise<Result<TData>> {
    loading.value = true;
    error.value = null;
    called.value = true;

    try {
      const result = await client.execute(operation, variables);

      if (isOk(result)) {
        data.value = result.value;
        options?.onCompleted?.(result.value);
      } else {
        error.value = result.error;
        options?.onError?.(result.error);
      }

      return result;
    } finally {
      loading.value = false;
    }
  }

  return [execute, { data, loading, error, called }];
}

/**
 * Subscription composable (placeholder for WebSocket support).
 */
export function useSubscription<TVariables extends Record<string, unknown>, TData>(
  _operation: TypedDocumentNode<TVariables, TData>,
  _variables: TVariables | Ref<TVariables>,
  _options?: {
    client?: BgqlClient;
    onData?: (data: TData) => void;
    onError?: (error: SdkError) => void;
  }
): {
  data: Ref<TData | null>;
  loading: Ref<boolean>;
  error: Ref<SdkError | null>;
} {
  // Subscription support requires WebSocket transport
  // This is a placeholder for future implementation
  throw new Error("Subscriptions are not yet supported. Coming soon!");
}
