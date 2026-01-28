/**
 * useQuery Composable for BGQL
 *
 * Vue composable for executing GraphQL queries with streaming support.
 */

import {
  ref,
  watch,
  onUnmounted,
  type Ref,
} from 'vue';
import type {
  DocumentNode,
  QueryOptions,
  StreamState,
  ExecutionController,
  Checkpoint,
  UseQueryResult,
  MultipartChunk,
} from './types';

/**
 * Default stream state.
 */
const DEFAULT_STREAM_STATE: StreamState = {
  hasNext: false,
  pendingDefers: [],
  activeStreams: [],
  progress: null,
};

/**
 * Executes a GraphQL query with full streaming support.
 *
 * @example
 * ```vue
 * <script setup>
 * import { useQuery } from '@bgql/client/vue'
 *
 * const { data, loading, error, streamState, pause, resume } = useQuery(
 *   gql`
 *     query GetUser($id: ID!) {
 *       user(id: $id) {
 *         id
 *         name
 *         ... @defer(label: "bio") {
 *           bio
 *           socialLinks
 *         }
 *       }
 *     }
 *   `,
 *   { variables: { id: '1' }, streaming: true }
 * )
 * </script>
 * ```
 */
export function useQuery<TData = unknown, TVariables = Record<string, unknown>>(
  query: DocumentNode | string,
  options: QueryOptions<TVariables> = {}
): UseQueryResult<TData> {
  const data = ref<TData | null>(null) as Ref<TData | null>;
  const loading = ref(!options.skip);
  const error = ref<Error | null>(null);
  const streamState = ref<StreamState>({ ...DEFAULT_STREAM_STATE });
  const controller = ref<ExecutionController | null>(null);

  let abortController: AbortController | null = null;
  let pollTimeoutId: ReturnType<typeof setTimeout> | null = null;

  const executeQuery = async (
    variables?: TVariables
  ): Promise<void> => {
    if (options.skip) {
      loading.value = false;
      return;
    }

    // Cancel any previous request
    abortController?.abort();
    abortController = new AbortController();

    loading.value = true;
    error.value = null;
    streamState.value = { ...DEFAULT_STREAM_STATE };

    const queryString = typeof query === 'string' ? query : getQueryString(query);
    const vars = variables ?? options.variables;

    try {
      if (options.streaming) {
        await executeStreamingQuery(queryString, vars, abortController.signal);
      } else {
        await executeSimpleQuery(queryString, vars, abortController.signal);
      }
    } catch (err) {
      if (err instanceof Error && err.name !== 'AbortError') {
        error.value = err;
        options.onError?.(err);
      }
    } finally {
      loading.value = streamState.value.hasNext;
    }
  };

  const executeSimpleQuery = async (
    queryString: string,
    variables: TVariables | undefined,
    signal: AbortSignal
  ): Promise<void> => {
    const response = await fetch(getBgqlEndpoint(), {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        Accept: 'application/json',
      },
      body: JSON.stringify({
        query: queryString,
        variables,
      }),
      signal,
    });

    if (!response.ok) {
      throw new Error(`HTTP ${response.status}: ${response.statusText}`);
    }

    const result = await response.json();

    if (result.errors?.length) {
      throw new Error(result.errors[0].message);
    }

    data.value = result.data;
    options.onData?.(result.data);
    options.onComplete?.();
  };

  const executeStreamingQuery = async (
    queryString: string,
    variables: TVariables | undefined,
    signal: AbortSignal
  ): Promise<void> => {
    const response = await fetch(getBgqlEndpoint(), {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        Accept: 'multipart/mixed; deferSpec=20220824, application/json',
      },
      body: JSON.stringify({
        query: queryString,
        variables,
      }),
      signal,
    });

    if (!response.ok) {
      throw new Error(`HTTP ${response.status}: ${response.statusText}`);
    }

    const contentType = response.headers.get('Content-Type') ?? '';

    if (contentType.includes('multipart/mixed')) {
      await processMultipartResponse(response, signal);
    } else {
      // Fallback to simple response
      const result = await response.json();
      if (result.errors?.length) {
        throw new Error(result.errors[0].message);
      }
      data.value = result.data;
      options.onData?.(result.data);
      options.onComplete?.();
    }
  };

  const processMultipartResponse = async (
    response: Response,
    signal: AbortSignal
  ): Promise<void> => {
    const reader = response.body?.getReader();
    if (!reader) {
      throw new Error('Response body is not readable');
    }

    const decoder = new TextDecoder();
    let buffer = '';
    const boundary = extractBoundary(response.headers.get('Content-Type') ?? '');

    try {
      while (!signal.aborted) {
        const { done, value } = await reader.read();

        if (done) {
          break;
        }

        buffer += decoder.decode(value, { stream: true });

        // Parse multipart chunks
        const chunks = parseMultipartBuffer(buffer, boundary);
        buffer = chunks.remaining;

        for (const chunk of chunks.parts) {
          processChunk(chunk as MultipartChunk<Partial<TData>>);
        }
      }
    } finally {
      reader.releaseLock();
    }

    options.onComplete?.();
  };

  const processChunk = (chunk: MultipartChunk<Partial<TData>>): void => {
    if (chunk.errors?.length) {
      error.value = new Error(chunk.errors[0].message);
      options.onError?.(error.value);
      return;
    }

    if (chunk.data) {
      if (chunk.path && chunk.path.length > 0) {
        // Incremental update - merge at path
        data.value = mergeAtPath(data.value, chunk.path, chunk.data);
      } else {
        // Initial data
        data.value = chunk.data as TData;
      }
      options.onData?.(data.value);
    }

    // Update stream state
    streamState.value = {
      ...streamState.value,
      hasNext: chunk.hasNext,
      pendingDefers: chunk.label
        ? streamState.value.pendingDefers.filter((l) => l !== chunk.label)
        : streamState.value.pendingDefers,
    };

    if (!chunk.hasNext) {
      loading.value = false;
    }
  };

  const pause = async (): Promise<string | null> => {
    if (controller.value) {
      return await controller.value.pause();
    }
    abortController?.abort();
    return null;
  };

  const resume = async (token?: string): Promise<void> => {
    if (token && controller.value) {
      await controller.value.resume(token);
    } else {
      await executeQuery(options.variables);
    }
  };

  const refetch = async (
    variables?: Record<string, unknown>
  ): Promise<void> => {
    await executeQuery((variables ?? options.variables) as TVariables);
  };

  // Initial execution
  if (!options.skip) {
    executeQuery(options.variables);
  }

  // Watch for variable changes
  if (options.variables) {
    watch(
      () => options.variables,
      (newVars) => {
        if (newVars && !options.skip) {
          executeQuery(newVars);
        }
      },
      { deep: true }
    );
  }

  // Set up polling
  if (options.pollInterval && options.pollInterval > 0) {
    const poll = (): void => {
      pollTimeoutId = setTimeout(async () => {
        if (!options.skip) {
          await executeQuery(options.variables);
          poll();
        }
      }, options.pollInterval);
    };
    poll();
  }

  // Cleanup
  onUnmounted(() => {
    abortController?.abort();
    if (pollTimeoutId) {
      clearTimeout(pollTimeoutId);
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
    get streamState() {
      return streamState.value;
    },
    get controller() {
      return controller.value;
    },
    pause,
    resume,
    refetch,
  };
}

// =============================================================================
// Helper Functions
// =============================================================================

function getBgqlEndpoint(): string {
  // Try to get from global config, fallback to default
  if (typeof window !== 'undefined' && (window as unknown as { __BGQL_ENDPOINT__?: string }).__BGQL_ENDPOINT__) {
    return (window as unknown as { __BGQL_ENDPOINT__?: string }).__BGQL_ENDPOINT__!;
  }
  return '/graphql';
}

function getQueryString(doc: DocumentNode): string {
  // In production, this would use graphql-tag's print function
  // For now, assume it has a loc.source.body
  const source = (doc as unknown as { loc?: { source?: { body?: string } } }).loc?.source?.body;
  if (source) {
    return source;
  }
  throw new Error('Cannot extract query string from DocumentNode');
}

function extractBoundary(contentType: string): string {
  const match = contentType.match(/boundary=(?:"([^"]+)"|([^;]+))/);
  return match?.[1] ?? match?.[2] ?? '-';
}

interface ParsedMultipart<T> {
  parts: MultipartChunk<T>[];
  remaining: string;
}

function parseMultipartBuffer<T>(
  buffer: string,
  boundary: string
): ParsedMultipart<T> {
  const parts: MultipartChunk<T>[] = [];
  const delimiter = `--${boundary}`;
  const segments = buffer.split(delimiter);

  // Keep the last incomplete segment
  const remaining = segments.pop() ?? '';

  for (const segment of segments) {
    if (!segment.trim() || segment.trim() === '--') {
      continue;
    }

    // Find the JSON body after headers
    const bodyStart = segment.indexOf('\r\n\r\n');
    if (bodyStart === -1) {
      continue;
    }

    const body = segment.slice(bodyStart + 4).trim();
    if (!body) {
      continue;
    }

    try {
      const json = JSON.parse(body);
      parts.push({
        data: json.data,
        path: json.path,
        label: json.label,
        hasNext: json.hasNext ?? false,
        errors: json.errors,
      });
    } catch {
      // Invalid JSON, skip
    }
  }

  return { parts, remaining: delimiter + remaining };
}

function mergeAtPath<T>(
  target: T | null,
  path: ReadonlyArray<string | number>,
  value: unknown
): T {
  if (target === null) {
    return value as T;
  }

  const result = structuredClone(target) as Record<string, unknown>;
  let current: Record<string, unknown> = result;

  for (let i = 0; i < path.length - 1; i++) {
    const key = path[i];
    if (typeof key === 'string') {
      if (!(key in current)) {
        current[key] = {};
      }
      current = current[key] as Record<string, unknown>;
    } else {
      // Array index
      if (!Array.isArray(current)) {
        throw new Error(`Expected array at path index ${i}`);
      }
      current = current[key] as Record<string, unknown>;
    }
  }

  const lastKey = path[path.length - 1];
  if (typeof lastKey === 'string') {
    current[lastKey] = value;
  } else if (Array.isArray(current)) {
    (current as unknown[])[lastKey] = value;
  }

  return result as T;
}
