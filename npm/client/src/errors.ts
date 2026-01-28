/**
 * Error types for bgql client operations.
 * All errors have a discriminant __typename for type-safe pattern matching.
 */

/**
 * Base error interface that all bgql errors extend.
 */
export interface BgqlError {
  readonly __typename: string;
  readonly message: string;
  readonly code: string;
}

/**
 * Network error - connection failed, timeout, etc.
 */
export interface NetworkError extends BgqlError {
  readonly __typename: 'NetworkError';
  readonly code: 'NETWORK_ERROR';
  readonly cause?: Error;
  readonly retryable: boolean;
}

/**
 * GraphQL validation error from the server.
 */
export interface GraphQLValidationError extends BgqlError {
  readonly __typename: 'GraphQLValidationError';
  readonly code: 'GRAPHQL_VALIDATION_ERROR';
  readonly locations?: ReadonlyArray<{ line: number; column: number }>;
  readonly path?: ReadonlyArray<string | number>;
}

/**
 * GraphQL execution error from the server.
 */
export interface GraphQLExecutionError extends BgqlError {
  readonly __typename: 'GraphQLExecutionError';
  readonly code: 'GRAPHQL_EXECUTION_ERROR';
  readonly locations?: ReadonlyArray<{ line: number; column: number }>;
  readonly path?: ReadonlyArray<string | number>;
  readonly extensions?: Record<string, unknown>;
}

/**
 * Request was aborted (via AbortController).
 */
export interface AbortError extends BgqlError {
  readonly __typename: 'AbortError';
  readonly code: 'ABORTED';
}

/**
 * Request timed out.
 */
export interface TimeoutError extends BgqlError {
  readonly __typename: 'TimeoutError';
  readonly code: 'TIMEOUT';
  readonly timeoutMs: number;
}

/**
 * Authentication required or failed.
 */
export interface AuthenticationError extends BgqlError {
  readonly __typename: 'AuthenticationError';
  readonly code: 'AUTHENTICATION_REQUIRED' | 'AUTHENTICATION_FAILED';
}

/**
 * Authorization failed - user doesn't have permission.
 */
export interface AuthorizationError extends BgqlError {
  readonly __typename: 'AuthorizationError';
  readonly code: 'FORBIDDEN';
  readonly requiredPermission?: string;
}

/**
 * Rate limit exceeded.
 */
export interface RateLimitError extends BgqlError {
  readonly __typename: 'RateLimitError';
  readonly code: 'RATE_LIMIT_EXCEEDED';
  readonly retryAfterMs: number;
}

/**
 * Unknown/unexpected error.
 */
export interface UnknownError extends BgqlError {
  readonly __typename: 'UnknownError';
  readonly code: 'UNKNOWN';
  readonly cause?: unknown;
}

/**
 * Union of all client-side errors.
 */
export type ClientError =
  | NetworkError
  | GraphQLValidationError
  | GraphQLExecutionError
  | AbortError
  | TimeoutError
  | AuthenticationError
  | AuthorizationError
  | RateLimitError
  | UnknownError;

// =============================================================================
// Error Constructors
// =============================================================================

export function networkError(
  message: string,
  options?: { cause?: Error; retryable?: boolean }
): NetworkError {
  return {
    __typename: 'NetworkError',
    message,
    code: 'NETWORK_ERROR',
    cause: options?.cause,
    retryable: options?.retryable ?? true,
  };
}

export function graphqlValidationError(
  message: string,
  options?: {
    locations?: ReadonlyArray<{ line: number; column: number }>;
    path?: ReadonlyArray<string | number>;
  }
): GraphQLValidationError {
  return {
    __typename: 'GraphQLValidationError',
    message,
    code: 'GRAPHQL_VALIDATION_ERROR',
    locations: options?.locations,
    path: options?.path,
  };
}

export function graphqlExecutionError(
  message: string,
  options?: {
    locations?: ReadonlyArray<{ line: number; column: number }>;
    path?: ReadonlyArray<string | number>;
    extensions?: Record<string, unknown>;
  }
): GraphQLExecutionError {
  return {
    __typename: 'GraphQLExecutionError',
    message,
    code: 'GRAPHQL_EXECUTION_ERROR',
    locations: options?.locations,
    path: options?.path,
    extensions: options?.extensions,
  };
}

export function abortError(message = 'Request was aborted'): AbortError {
  return {
    __typename: 'AbortError',
    message,
    code: 'ABORTED',
  };
}

export function timeoutError(timeoutMs: number): TimeoutError {
  return {
    __typename: 'TimeoutError',
    message: `Request timed out after ${timeoutMs}ms`,
    code: 'TIMEOUT',
    timeoutMs,
  };
}

export function authenticationError(
  message: string,
  failed = false
): AuthenticationError {
  return {
    __typename: 'AuthenticationError',
    message,
    code: failed ? 'AUTHENTICATION_FAILED' : 'AUTHENTICATION_REQUIRED',
  };
}

export function authorizationError(
  message: string,
  requiredPermission?: string
): AuthorizationError {
  return {
    __typename: 'AuthorizationError',
    message,
    code: 'FORBIDDEN',
    requiredPermission,
  };
}

export function rateLimitError(retryAfterMs: number): RateLimitError {
  return {
    __typename: 'RateLimitError',
    message: `Rate limit exceeded. Retry after ${retryAfterMs}ms`,
    code: 'RATE_LIMIT_EXCEEDED',
    retryAfterMs,
  };
}

export function unknownError(message: string, cause?: unknown): UnknownError {
  return {
    __typename: 'UnknownError',
    message,
    code: 'UNKNOWN',
    cause,
  };
}

// =============================================================================
// Type Guards
// =============================================================================

export function isNetworkError(error: ClientError): error is NetworkError {
  return error.__typename === 'NetworkError';
}

export function isGraphQLValidationError(
  error: ClientError
): error is GraphQLValidationError {
  return error.__typename === 'GraphQLValidationError';
}

export function isGraphQLExecutionError(
  error: ClientError
): error is GraphQLExecutionError {
  return error.__typename === 'GraphQLExecutionError';
}

export function isAbortError(error: ClientError): error is AbortError {
  return error.__typename === 'AbortError';
}

export function isTimeoutError(error: ClientError): error is TimeoutError {
  return error.__typename === 'TimeoutError';
}

export function isRetryable(error: ClientError): boolean {
  switch (error.__typename) {
    case 'NetworkError':
      return error.retryable;
    case 'RateLimitError':
    case 'TimeoutError':
      return true;
    default:
      return false;
  }
}
