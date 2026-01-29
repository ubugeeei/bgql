/**
 * Strongly typed GraphQL client.
 *
 * Provides compile-time type safety for GraphQL operations.
 */

import {
  type SdkError,
  ErrorCode,
  sdkError,
  isRetryable,
  type GraphQLError,
} from "./error";
import { ok, err, type AsyncResult } from "./result";

/**
 * Operation kind.
 */
export type OperationKind = "query" | "mutation" | "subscription";

/**
 * Typed GraphQL operation definition.
 *
 * @example
 * ```ts
 * const GetUser = {
 *   operation: "query GetUser($id: ID!) { user(id: $id) { id name } }",
 *   operationName: "GetUser",
 *   kind: "query",
 * } as const satisfies TypedOperation<GetUserVariables, GetUserData>;
 * ```
 */
export interface TypedOperation<
  TVariables extends Record<string, unknown> = Record<string, unknown>,
  TData = unknown
> {
  readonly operation: string;
  readonly operationName: string;
  readonly kind: OperationKind;
  /** Phantom types for inference */
  readonly __variables?: TVariables;
  readonly __data?: TData;
}

/**
 * Extract variables type from operation.
 */
export type VariablesOf<T> = T extends TypedOperation<infer V, unknown>
  ? V
  : never;

/**
 * Extract data type from operation.
 */
export type DataOf<T> = T extends TypedOperation<Record<string, unknown>, infer D> ? D : never;

/**
 * GraphQL response structure.
 */
export interface GraphQLResponse<T = unknown> {
  readonly data?: T | null;
  readonly errors?: readonly GraphQLError[];
}

/**
 * Client configuration.
 */
export interface ClientConfig {
  readonly url: string;
  readonly timeout?: number;
  readonly maxRetries?: number;
  readonly retryDelayMs?: number;
  readonly headers?: Record<string, string>;
}

/**
 * Request options.
 */
export interface RequestOptions {
  readonly headers?: Record<string, string>;
  readonly signal?: AbortSignal;
}

/**
 * Fetch function type.
 */
type FetchFn = typeof fetch;

/**
 * Strongly typed GraphQL client.
 */
export class BgqlClient {
  private readonly config: Required<Omit<ClientConfig, "headers">> & {
    headers: Record<string, string>;
  };
  private readonly fetchFn: FetchFn;

  constructor(config: ClientConfig, fetchFn: FetchFn = fetch) {
    this.config = {
      url: config.url,
      timeout: config.timeout ?? 30000,
      maxRetries: config.maxRetries ?? 3,
      retryDelayMs: config.retryDelayMs ?? 100,
      headers: config.headers ?? {},
    };
    this.fetchFn = fetchFn;
  }

  /**
   * Executes a typed operation.
   */
  async execute<TOp extends TypedOperation>(
    operation: TOp,
    variables: VariablesOf<TOp>,
    options?: RequestOptions
  ): AsyncResult<DataOf<TOp>> {
    const result = await this.executeRaw<DataOf<TOp>>(
      operation.operation,
      variables,
      operation.operationName,
      options
    );

    if (result._tag === "Err") {
      return result;
    }

    const response = result.value;

    if (response.errors && response.errors.length > 0) {
      const firstError = response.errors[0];
      return err(
        sdkError(ErrorCode.ExecutionError, firstError?.message ?? "Unknown error", {
          extensions: { graphqlErrors: response.errors },
        })
      );
    }

    if (response.data === null || response.data === undefined) {
      return err(sdkError(ErrorCode.NoData, "No data in response"));
    }

    return ok(response.data);
  }

  /**
   * Executes a raw GraphQL request.
   */
  async executeRaw<T>(
    query: string,
    variables?: Record<string, unknown>,
    operationName?: string,
    options?: RequestOptions
  ): AsyncResult<GraphQLResponse<T>> {
    let lastError: SdkError = sdkError(
      ErrorCode.NetworkError,
      "No attempts made"
    );

    for (let attempt = 0; attempt <= this.config.maxRetries; attempt++) {
      if (attempt > 0) {
        const delay = this.config.retryDelayMs * Math.pow(2, attempt - 1);
        await this.sleep(delay);
      }

      const result = await this.doRequest<T>(
        query,
        variables,
        operationName,
        options
      );

      if (result._tag === "Ok") {
        return result;
      }

      lastError = result.error;

      if (!isRetryable(lastError.code)) {
        return result;
      }
    }

    return err(lastError);
  }

  private async doRequest<T>(
    query: string,
    variables?: Record<string, unknown>,
    operationName?: string,
    options?: RequestOptions
  ): AsyncResult<GraphQLResponse<T>> {
    const headers = {
      "Content-Type": "application/json",
      ...this.config.headers,
      ...options?.headers,
    };

    const body = JSON.stringify({
      query,
      variables,
      operationName,
    });

    const controller = new AbortController();
    const timeoutId = setTimeout(
      () => controller.abort(),
      this.config.timeout
    );

    try {
      const response = await this.fetchFn(this.config.url, {
        method: "POST",
        headers,
        body,
        signal: options?.signal ?? controller.signal,
      });

      clearTimeout(timeoutId);

      if (!response.ok) {
        return err(
          sdkError(ErrorCode.HttpError, `HTTP ${response.status}`, {
            extensions: { status: response.status },
          })
        );
      }

      const json = (await response.json()) as GraphQLResponse<T>;
      return ok(json);
    } catch (error) {
      clearTimeout(timeoutId);

      if (error instanceof Error) {
        if (error.name === "AbortError") {
          return err(sdkError(ErrorCode.Timeout, "Request timed out"));
        }
        return err(sdkError(ErrorCode.NetworkError, error.message));
      }

      return err(sdkError(ErrorCode.NetworkError, "Unknown error"));
    }
  }

  private sleep(ms: number): Promise<void> {
    return new Promise((resolve) => setTimeout(resolve, ms));
  }
}

/**
 * Creates a typed client.
 */
export function createClient(
  config: ClientConfig,
  fetchFn?: FetchFn
): BgqlClient {
  return new BgqlClient(config, fetchFn);
}

/**
 * Operation builder for type-safe operation definitions.
 */
export function defineOperation<
  TVariables extends Record<string, unknown> = Record<string, unknown>,
  TData = unknown
>(
  kind: OperationKind,
  operationName: string,
  operation: string
): TypedOperation<TVariables, TData> {
  return {
    operation,
    operationName,
    kind,
  } as TypedOperation<TVariables, TData>;
}

/**
 * Query operation builder.
 */
export function defineQuery<
  TVariables extends Record<string, unknown> = Record<string, unknown>,
  TData = unknown
>(
  operationName: string,
  operation: string
): TypedOperation<TVariables, TData> {
  return defineOperation("query", operationName, operation);
}

/**
 * Mutation operation builder.
 */
export function defineMutation<
  TVariables extends Record<string, unknown> = Record<string, unknown>,
  TData = unknown
>(
  operationName: string,
  operation: string
): TypedOperation<TVariables, TData> {
  return defineOperation("mutation", operationName, operation);
}

// Re-exports
export { type SdkError, ErrorCode, type GraphQLError } from "./error";
export { type Result, type AsyncResult, ok, err, isOk, isErr } from "./result";
