/**
 * Strongly typed error system for Better GraphQL SDK.
 *
 * Provides compile-time guarantees for error handling with discriminated unions.
 */

/**
 * Typed error codes - enables exhaustive pattern matching.
 */
export const ErrorCode = {
  // Network errors
  NetworkError: "NETWORK_ERROR",
  Timeout: "TIMEOUT",
  ConnectionRefused: "CONNECTION_REFUSED",

  // Protocol errors
  HttpError: "HTTP_ERROR",
  InvalidUrl: "INVALID_URL",
  InvalidResponse: "INVALID_RESPONSE",

  // GraphQL errors
  ParseError: "PARSE_ERROR",
  ValidationError: "VALIDATION_ERROR",
  ExecutionError: "EXECUTION_ERROR",
  NoOperation: "NO_OPERATION",
  NoData: "NO_DATA",

  // Auth errors
  AuthError: "AUTH_ERROR",
  Unauthorized: "UNAUTHORIZED",
  Forbidden: "FORBIDDEN",

  // Resource errors
  NotFound: "NOT_FOUND",
  Conflict: "CONFLICT",

  // Internal errors
  InternalError: "INTERNAL_ERROR",
} as const;

export type ErrorCode = (typeof ErrorCode)[keyof typeof ErrorCode];

/**
 * Check if an error code is retryable.
 */
export function isRetryable(code: ErrorCode): boolean {
  return (
    code === ErrorCode.NetworkError ||
    code === ErrorCode.Timeout ||
    code === ErrorCode.ConnectionRefused
  );
}

/**
 * Check if an error code represents a client error.
 */
export function isClientError(code: ErrorCode): boolean {
  return (
    code === ErrorCode.ParseError ||
    code === ErrorCode.ValidationError ||
    code === ErrorCode.AuthError ||
    code === ErrorCode.Unauthorized ||
    code === ErrorCode.Forbidden ||
    code === ErrorCode.NotFound ||
    code === ErrorCode.InvalidUrl ||
    code === ErrorCode.NoOperation
  );
}

/**
 * Check if an error code represents a server error.
 */
export function isServerError(code: ErrorCode): boolean {
  return (
    code === ErrorCode.InternalError || code === ErrorCode.ExecutionError
  );
}

/**
 * Strongly typed SDK error.
 */
export interface SdkError<C extends ErrorCode = ErrorCode> {
  readonly _tag: "SdkError";
  readonly code: C;
  readonly message: string;
  readonly cause?: SdkError;
  readonly extensions?: Record<string, unknown>;
}

/**
 * Creates a new SDK error.
 */
export function sdkError<C extends ErrorCode>(
  code: C,
  message: string,
  options?: { cause?: SdkError; extensions?: Record<string, unknown> }
): SdkError<C> {
  const result: SdkError<C> = {
    _tag: "SdkError",
    code,
    message,
  };
  if (options?.cause !== undefined) {
    (result as { cause: SdkError }).cause = options.cause;
  }
  if (options?.extensions !== undefined) {
    (result as { extensions: Record<string, unknown> }).extensions = options.extensions;
  }
  return result;
}

// Convenience constructors
export const SdkError = {
  network: (message: string) =>
    sdkError(ErrorCode.NetworkError, message),
  timeout: () => sdkError(ErrorCode.Timeout, "Request timed out"),
  parse: (message: string) => sdkError(ErrorCode.ParseError, message),
  validation: (message: string) =>
    sdkError(ErrorCode.ValidationError, message),
  auth: (message: string) => sdkError(ErrorCode.AuthError, message),
  notFound: (resource: string) =>
    sdkError(ErrorCode.NotFound, `${resource} not found`),
  internal: (message: string) =>
    sdkError(ErrorCode.InternalError, message),
} as const;

/**
 * Type guard for SdkError.
 */
export function isSdkError(value: unknown): value is SdkError {
  return (
    typeof value === "object" &&
    value !== null &&
    "_tag" in value &&
    value._tag === "SdkError"
  );
}

/**
 * GraphQL error from server response.
 */
export interface GraphQLError {
  readonly message: string;
  readonly path?: readonly (string | number)[];
  readonly locations?: readonly { line: number; column: number }[];
  readonly extensions?: Record<string, unknown>;
}

/**
 * Type guard for GraphQL errors.
 */
export function isGraphQLError(value: unknown): value is GraphQLError {
  return (
    typeof value === "object" &&
    value !== null &&
    "message" in value &&
    typeof (value as GraphQLError).message === "string"
  );
}
