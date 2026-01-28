/**
 * Vue SDK Types for BGQL
 *
 * Framework-agnostic component model with streaming support.
 */

import type { GraphQLResponse, RequestOptions } from '../types';

/**
 * Document node type (compatible with graphql-tag and gql)
 */
export interface DocumentNode {
  readonly kind: 'Document';
  readonly definitions: ReadonlyArray<unknown>;
  readonly loc?: unknown;
}

/**
 * Query options for useQuery composable.
 */
export interface QueryOptions<TVariables = Record<string, unknown>> {
  /**
   * Variables for the query.
   */
  readonly variables?: TVariables;

  /**
   * Skip execution of the query.
   */
  readonly skip?: boolean;

  /**
   * Polling interval in milliseconds.
   */
  readonly pollInterval?: number;

  /**
   * Fetch policy.
   */
  readonly fetchPolicy?: FetchPolicy;

  /**
   * Request options.
   */
  readonly requestOptions?: RequestOptions;

  /**
   * Enable streaming (@defer/@stream) support.
   */
  readonly streaming?: boolean;

  /**
   * Callback when data is received.
   */
  readonly onData?: (data: unknown) => void;

  /**
   * Callback when an error occurs.
   */
  readonly onError?: (error: Error) => void;

  /**
   * Callback when streaming completes.
   */
  readonly onComplete?: () => void;
}

/**
 * Fetch policy for caching behavior.
 */
export type FetchPolicy =
  | 'cache-first'
  | 'cache-only'
  | 'cache-and-network'
  | 'network-only'
  | 'no-cache';

/**
 * Stream state for tracking @defer/@stream progress.
 */
export interface StreamState {
  /**
   * Whether there are more chunks coming.
   */
  readonly hasNext: boolean;

  /**
   * Labels of pending deferred fragments.
   */
  readonly pendingDefers: ReadonlyArray<string>;

  /**
   * Labels of active streams.
   */
  readonly activeStreams: ReadonlyArray<string>;

  /**
   * Overall streaming progress (0-1).
   */
  readonly progress: number | null;
}

/**
 * Execution controller for pause/resume.
 */
export interface ExecutionController {
  /**
   * Pause the current execution.
   * Returns a resume token.
   */
  pause(): Promise<string>;

  /**
   * Resume from a token.
   */
  resume(token: string): Promise<void>;

  /**
   * Cancel the execution.
   */
  cancel(): void;

  /**
   * Get the current checkpoint.
   */
  getCheckpoint(): Promise<Checkpoint | null>;
}

/**
 * Checkpoint for resumable queries.
 */
export interface Checkpoint {
  readonly id: string;
  readonly position: ExecutionPosition;
  readonly timestamp: Date;
  readonly dataSnapshot: unknown;
  readonly resumeToken: string;
}

/**
 * Execution position within a query.
 */
export interface ExecutionPosition {
  readonly path: ReadonlyArray<string | number>;
  readonly streamCursor?: string;
  readonly binaryOffset?: number;
}

/**
 * Binary stream handle for media content.
 */
export interface BinaryStreamHandle {
  readonly id: string;
  readonly contentType: string;
  readonly totalSize?: number;
  readonly chunkSize: number;
  readonly supportsRange: boolean;
  readonly supportsPause: boolean;
}

/**
 * Binary stream state.
 */
export interface BinaryStreamState {
  /**
   * Current playback/download position in bytes.
   */
  readonly position: number;

  /**
   * Buffered ranges.
   */
  readonly buffered: ReadonlyArray<BufferedRange>;

  /**
   * Whether the stream is paused.
   */
  readonly paused: boolean;

  /**
   * Whether the stream has ended.
   */
  readonly ended: boolean;

  /**
   * Current error if any.
   */
  readonly error: Error | null;

  /**
   * Download/stream progress (0-1).
   */
  readonly progress: number;
}

/**
 * Buffered byte range.
 */
export interface BufferedRange {
  readonly start: number;
  readonly end: number;
}

/**
 * Server fragment options.
 */
export interface ServerFragmentOptions {
  /**
   * Fragment name in the schema.
   */
  readonly fragmentName: string;

  /**
   * Variables for the fragment.
   */
  readonly variables?: Record<string, unknown>;

  /**
   * Cache strategy.
   */
  readonly cache?: CacheStrategy;

  /**
   * Whether to prerender.
   */
  readonly prerender?: boolean;
}

/**
 * Cache strategy for server fragments.
 */
export type CacheStrategy = 'none' | 'request' | 'user' | 'global';

/**
 * Hydration strategy.
 */
export type HydrationStrategy =
  | 'immediate'
  | 'idle'
  | 'visible'
  | 'interaction'
  | 'never';

/**
 * Hydration priority.
 */
export type HydrationPriority = 'critical' | 'high' | 'normal' | 'low';

/**
 * Multipart response chunk for streaming.
 */
export interface MultipartChunk<TData = unknown> {
  readonly data?: TData;
  readonly path?: ReadonlyArray<string | number>;
  readonly label?: string;
  readonly hasNext: boolean;
  readonly errors?: ReadonlyArray<{ message: string }>;
}

/**
 * BGQL query context provided to child components.
 */
export interface BgqlQueryContext<TData = unknown> {
  readonly data: TData | null;
  readonly loading: boolean;
  readonly error: Error | null;
  readonly streamState: StreamState;
  readonly controller: ExecutionController | null;
}

/**
 * Result of useQuery composable.
 */
export interface UseQueryResult<TData> {
  readonly data: TData | null;
  readonly loading: boolean;
  readonly error: Error | null;
  readonly streamState: StreamState;
  readonly controller: ExecutionController | null;
  readonly pause: () => Promise<string | null>;
  readonly resume: (token?: string) => Promise<void>;
  readonly refetch: (variables?: Record<string, unknown>) => Promise<void>;
}

/**
 * Result of useServerFragment composable.
 */
export interface UseServerFragmentResult<TData> {
  readonly data: TData | null;
  readonly loading: boolean;
  readonly error: Error | null;
}

/**
 * Result of useBinaryStream composable.
 */
export interface UseBinaryStreamResult {
  readonly state: BinaryStreamState;
  readonly play: () => void;
  readonly pause: () => void;
  readonly seek: (position: number) => void;
  readonly stop: () => void;
}
