/**
 * Core types for the bgql client.
 */

/**
 * Configuration options for creating a bgql client.
 */
export interface ClientConfig {
  /**
   * The GraphQL endpoint URL.
   */
  readonly url: string;

  /**
   * Default headers to include in every request.
   */
  readonly headers?: Record<string, string>;

  /**
   * Timeout for requests in milliseconds.
   * @default 30000
   */
  readonly timeout?: number;

  /**
   * Fetch implementation to use.
   * @default globalThis.fetch
   */
  readonly fetch?: typeof fetch;

  /**
   * Retry configuration.
   */
  readonly retry?: RetryConfig;

  /**
   * Custom error handler for logging/reporting.
   */
  readonly onError?: (error: unknown) => void;

  /**
   * Credentials mode for fetch.
   * @default 'same-origin'
   */
  readonly credentials?: RequestCredentials;
}

/**
 * Retry configuration.
 */
export interface RetryConfig {
  /**
   * Maximum number of retries.
   * @default 3
   */
  readonly maxRetries?: number;

  /**
   * Initial delay between retries in milliseconds.
   * @default 1000
   */
  readonly initialDelayMs?: number;

  /**
   * Maximum delay between retries in milliseconds.
   * @default 30000
   */
  readonly maxDelayMs?: number;

  /**
   * Whether to use exponential backoff.
   * @default true
   */
  readonly exponentialBackoff?: boolean;

  /**
   * Custom retry condition.
   */
  readonly shouldRetry?: (error: unknown, attempt: number) => boolean;
}

/**
 * Options for a single operation request.
 */
export interface RequestOptions {
  /**
   * AbortSignal for cancellation.
   */
  readonly signal?: AbortSignal;

  /**
   * Additional headers for this request.
   */
  readonly headers?: Record<string, string>;

  /**
   * Timeout override for this request.
   */
  readonly timeout?: number;

  /**
   * Context passed to middleware.
   */
  readonly context?: Record<string, unknown>;
}

/**
 * A GraphQL operation (query, mutation, subscription).
 */
export interface Operation<TVariables, TData> {
  /**
   * The operation name.
   */
  readonly operationName: string;

  /**
   * The operation type.
   */
  readonly operationType: 'query' | 'mutation' | 'subscription';

  /**
   * The GraphQL document string.
   */
  readonly document: string;

  /**
   * Variables for the operation.
   */
  readonly variables?: TVariables;
}

/**
 * GraphQL response from the server.
 */
export interface GraphQLResponse<TData> {
  readonly data?: TData;
  readonly errors?: ReadonlyArray<GraphQLErrorResponse>;
  readonly extensions?: Record<string, unknown>;
}

/**
 * GraphQL error in the response.
 */
export interface GraphQLErrorResponse {
  readonly message: string;
  readonly locations?: ReadonlyArray<{ line: number; column: number }>;
  readonly path?: ReadonlyArray<string | number>;
  readonly extensions?: Record<string, unknown>;
}

/**
 * Partial data with deferred fields.
 */
export type PartialData<T> = {
  [K in keyof T]: T[K] extends object
    ? PartialData<T[K]> | Promise<T[K]>
    : T[K];
};

/**
 * Streaming result for @defer/@stream.
 */
export interface StreamingResult<TData> {
  /**
   * Initial data (may have deferred fields).
   */
  readonly data: PartialData<TData>;

  /**
   * Whether there are more chunks coming.
   */
  readonly hasNext: boolean;

  /**
   * Async iterator for incremental updates.
   */
  readonly incremental?: AsyncIterable<IncrementalUpdate<TData>>;
}

/**
 * Incremental update from @defer/@stream.
 */
export interface IncrementalUpdate<TData> {
  readonly path: ReadonlyArray<string | number>;
  readonly data: Partial<TData>;
  readonly hasNext: boolean;
}

/**
 * Subscription observable.
 */
export interface Subscription<TData> {
  /**
   * Subscribe to updates.
   */
  subscribe(handlers: {
    next: (data: TData) => void;
    error?: (error: unknown) => void;
    complete?: () => void;
  }): Unsubscribe;
}

/**
 * Function to unsubscribe from a subscription.
 */
export type Unsubscribe = () => void;

/**
 * Middleware function for request/response interception.
 */
export type Middleware = (
  operation: Operation<unknown, unknown>,
  options: RequestOptions,
  next: (
    operation: Operation<unknown, unknown>,
    options: RequestOptions
  ) => Promise<GraphQLResponse<unknown>>
) => Promise<GraphQLResponse<unknown>>;

/**
 * Generated client operation methods.
 * This is a placeholder - actual types are generated from schema.
 */
export interface GeneratedOperations {
  [key: string]: (
    variables?: unknown,
    options?: RequestOptions
  ) => Promise<unknown>;
}

// =============================================================================
// Branded Types for Type-Safe IDs
// =============================================================================

/**
 * Creates a branded type for nominal typing.
 *
 * @example
 * ```typescript
 * type UserId = Brand<string, 'UserId'>;
 * type PostId = Brand<string, 'PostId'>;
 *
 * const userId: UserId = 'user_123' as UserId;
 * const postId: PostId = 'post_456' as PostId;
 *
 * // Error: Type 'UserId' is not assignable to type 'PostId'
 * const wrong: PostId = userId;
 * ```
 */
export type Brand<T, B extends string> = T & { readonly __brand: B };

/**
 * Helper to create branded values.
 */
export function brand<T, B extends string>(value: T): Brand<T, B> {
  return value as Brand<T, B>;
}

/**
 * Extracts the base type from a branded type.
 */
export type Unbrand<T> = T extends Brand<infer U, string> ? U : T;

// =============================================================================
// Option Type (for Option<T> from schema)
// =============================================================================

/**
 * Represents an optional value.
 * Maps to Option<T> in bgql schema.
 */
export type Option<T> = T | null;

/**
 * Checks if an Option has a value.
 */
export function isSome<T>(option: Option<T>): option is T {
  return option !== null;
}

/**
 * Checks if an Option is empty.
 */
export function isNone<T>(option: Option<T>): option is null {
  return option === null;
}

/**
 * Maps an Option value.
 */
export function mapOption<T, U>(
  option: Option<T>,
  fn: (value: T) => U
): Option<U> {
  return option !== null ? fn(option) : null;
}

/**
 * Unwraps an Option with a default value.
 */
export function unwrapOption<T>(option: Option<T>, defaultValue: T): T {
  return option !== null ? option : defaultValue;
}
