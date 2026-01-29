/**
 * useMutation Composable for BGQL
 *
 * Vue composable for executing GraphQL mutations.
 */

import {
  ref,
  inject,
  onUnmounted,
  type Ref,
} from 'vue';
import type {
  DocumentNode,
  RequestOptions,
} from './types';
import type { BgqlClient } from '../client';
import { BGQL_CLIENT_KEY } from './useQuery';

/**
 * Mutation options.
 */
export interface MutationOptions {
  /**
   * Custom client instance. If not provided, uses injected client.
   */
  readonly client?: BgqlClient;

  /**
   * Request options for the mutation.
   */
  readonly requestOptions?: RequestOptions;

  /**
   * Callback when mutation succeeds.
   */
  readonly onSuccess?: (data: unknown) => void;

  /**
   * Callback when mutation fails.
   */
  readonly onError?: (error: Error) => void;

  /**
   * Callback when mutation completes (success or error).
   */
  readonly onSettled?: () => void;

  /**
   * Refetch queries after mutation.
   * Array of query names or query documents to refetch.
   */
  readonly refetchQueries?: ReadonlyArray<string | DocumentNode>;
}

/**
 * Result of useMutation composable.
 */
export interface UseMutationResult<TData, TVariables> {
  /**
   * Execute the mutation with the given variables.
   */
  readonly mutate: (variables?: TVariables) => Promise<TData | null>;

  /**
   * Alias for mutate - execute the mutation.
   */
  readonly mutateAsync: (variables?: TVariables) => Promise<TData | null>;

  /**
   * The data returned from the mutation.
   */
  readonly data: TData | null;

  /**
   * Whether the mutation is currently executing.
   */
  readonly loading: boolean;

  /**
   * Error from the last mutation execution.
   */
  readonly error: Error | null;

  /**
   * Whether the mutation has been called at least once.
   */
  readonly called: boolean;

  /**
   * Reset the mutation state.
   */
  readonly reset: () => void;
}

/**
 * Executes a GraphQL mutation.
 *
 * The mutation is not executed automatically - you must call `mutate()`
 * with the variables. The state is reset between calls.
 *
 * @example
 * ```vue
 * <script setup>
 * import { useMutation } from '@bgql/client/vue'
 *
 * const { mutate, data, loading, error } = useMutation(
 *   gql`
 *     mutation CreateUser($input: CreateUserInput!) {
 *       createUser(input: $input) {
 *         id
 *         name
 *         email
 *       }
 *     }
 *   `,
 *   {
 *     onSuccess: (data) => {
 *       console.log('User created:', data)
 *     },
 *     onError: (error) => {
 *       console.error('Failed to create user:', error)
 *     },
 *   }
 * )
 *
 * async function handleSubmit(formData) {
 *   const result = await mutate({ input: formData })
 *   if (result) {
 *     // Navigate to user profile
 *     router.push(`/users/${result.createUser.id}`)
 *   }
 * }
 * </script>
 *
 * <template>
 *   <form @submit.prevent="handleSubmit">
 *     <button type="submit" :disabled="loading">
 *       {{ loading ? 'Creating...' : 'Create User' }}
 *     </button>
 *     <p v-if="error" class="error">{{ error.message }}</p>
 *   </form>
 * </template>
 * ```
 */
export function useMutation<TData = unknown, TVariables = Record<string, unknown>>(
  mutation: DocumentNode | string,
  options: MutationOptions = {}
): UseMutationResult<TData, TVariables> {
  // Get the injected client, or use provided client
  const injectedClient = inject<BgqlClient | null>(BGQL_CLIENT_KEY, null);
  const client = options.client ?? injectedClient;

  const data = ref<TData | null>(null) as Ref<TData | null>;
  const loading = ref(false);
  const error = ref<Error | null>(null);
  const called = ref(false);

  let abortController: AbortController | null = null;

  /**
   * Reset mutation state.
   */
  const reset = (): void => {
    abortController?.abort();
    data.value = null;
    loading.value = false;
    error.value = null;
    called.value = false;
  };

  /**
   * Execute the mutation.
   */
  const mutate = async (variables?: TVariables): Promise<TData | null> => {
    // Cancel any previous mutation
    abortController?.abort();
    abortController = new AbortController();

    // Reset state for new call
    data.value = null;
    error.value = null;
    loading.value = true;
    called.value = true;

    const mutationString = typeof mutation === 'string' ? mutation : getMutationString(mutation);

    try {
      const response = await fetch(getBgqlEndpoint(), {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          Accept: 'application/json',
          ...getGlobalHeaders(),
          ...options.requestOptions?.headers,
        },
        body: JSON.stringify({
          query: mutationString,
          variables,
        }),
        signal: abortController.signal,
      });

      if (!response.ok) {
        throw new Error(`HTTP ${response.status}: ${response.statusText}`);
      }

      const result = await response.json();

      if (result.errors?.length) {
        throw new Error(result.errors[0].message);
      }

      data.value = result.data;
      options.onSuccess?.(result.data);

      return result.data as TData;
    } catch (err) {
      if (err instanceof Error && err.name !== 'AbortError') {
        error.value = err;
        options.onError?.(err);
      }
      return null;
    } finally {
      loading.value = false;
      options.onSettled?.();
    }
  };

  // Cleanup on unmount
  onUnmounted(() => {
    abortController?.abort();
  });

  return {
    mutate,
    mutateAsync: mutate, // Alias
    get data() {
      return data.value;
    },
    get loading() {
      return loading.value;
    },
    get error() {
      return error.value;
    },
    get called() {
      return called.value;
    },
    reset,
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

function getGlobalHeaders(): Record<string, string> {
  if (typeof window !== 'undefined' && (window as unknown as { __BGQL_HEADERS__?: Record<string, string> }).__BGQL_HEADERS__) {
    return (window as unknown as { __BGQL_HEADERS__?: Record<string, string> }).__BGQL_HEADERS__!;
  }
  return {};
}

function getMutationString(doc: DocumentNode): string {
  const source = (doc as unknown as { loc?: { source?: { body?: string } } }).loc?.source?.body;
  if (source) {
    return source;
  }
  throw new Error('Cannot extract mutation string from DocumentNode');
}

export default useMutation;
