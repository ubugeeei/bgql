/**
 * bgql Client Implementation
 *
 * Type-safe GraphQL client with Result-based error handling.
 */

import {
  Result,
  ok,
  err,
} from './result';
import {
  ClientError,
  networkError,
  graphqlExecutionError,
  abortError,
  timeoutError,
  unknownError,
} from './errors';
import type {
  ClientConfig,
  RequestOptions,
  Operation,
  GraphQLResponse,
  Middleware,
  RetryConfig,
} from './types';

/**
 * Default client configuration.
 */
const DEFAULT_CONFIG: Required<
  Pick<ClientConfig, 'timeout' | 'credentials'>
> = {
  timeout: 30000,
  credentials: 'same-origin',
};

/**
 * Default retry configuration.
 */
const DEFAULT_RETRY_CONFIG: Required<RetryConfig> = {
  maxRetries: 3,
  initialDelayMs: 1000,
  maxDelayMs: 30000,
  exponentialBackoff: true,
  shouldRetry: () => true,
};

/**
 * Creates a new bgql client.
 *
 * @example
 * ```typescript
 * const client = createClient({
 *   url: 'https://api.example.com/graphql',
 *   headers: {
 *     'Authorization': `Bearer ${token}`,
 *   },
 * });
 *
 * const result = await client.execute({
 *   operationName: 'GetUser',
 *   operationType: 'query',
 *   document: `query GetUser($id: ID!) { user(id: $id) { id name } }`,
 *   variables: { id: '1' },
 * });
 *
 * if (result.ok) {
 *   console.log(result.value.user);
 * } else {
 *   console.error(result.error.message);
 * }
 * ```
 */
export function createClient(config: ClientConfig): BgqlClient {
  return new BgqlClientImpl(config);
}

/**
 * The bgql client interface.
 */
export interface BgqlClient {
  /**
   * Executes a GraphQL operation.
   */
  execute<TData, TVariables = Record<string, unknown>>(
    operation: Operation<TVariables, TData>,
    options?: RequestOptions
  ): Promise<Result<TData, ClientError>>;

  /**
   * Executes a raw GraphQL query.
   */
  query<TData, TVariables = Record<string, unknown>>(
    document: string,
    variables?: TVariables,
    options?: RequestOptions
  ): Promise<Result<TData, ClientError>>;

  /**
   * Executes a raw GraphQL mutation.
   */
  mutate<TData, TVariables = Record<string, unknown>>(
    document: string,
    variables?: TVariables,
    options?: RequestOptions
  ): Promise<Result<TData, ClientError>>;

  /**
   * Adds middleware to the client.
   */
  use(middleware: Middleware): BgqlClient;

  /**
   * Sets default headers for all requests.
   */
  setHeaders(headers: Record<string, string>): void;

  /**
   * Sets the authorization token.
   */
  setAuthToken(token: string | null): void;
}

/**
 * Internal client implementation.
 */
class BgqlClientImpl implements BgqlClient {
  private readonly config: ClientConfig;
  private readonly fetchFn: typeof fetch;
  private readonly middlewares: Middleware[] = [];
  private headers: Record<string, string>;

  constructor(config: ClientConfig) {
    this.config = config;
    this.fetchFn = config.fetch ?? globalThis.fetch.bind(globalThis);
    this.headers = { ...config.headers };
  }

  async execute<TData, TVariables = Record<string, unknown>>(
    operation: Operation<TVariables, TData>,
    options?: RequestOptions
  ): Promise<Result<TData, ClientError>> {
    // Build the request chain with middleware
    const chain = this.buildMiddlewareChain();

    try {
      const response = await chain(
        operation as Operation<unknown, unknown>,
        options ?? {}
      );
      return this.processResponse<TData>(response);
    } catch (error) {
      return err(this.handleError(error));
    }
  }

  async query<TData, TVariables = Record<string, unknown>>(
    document: string,
    variables?: TVariables,
    options?: RequestOptions
  ): Promise<Result<TData, ClientError>> {
    return this.execute<TData, TVariables>(
      {
        operationName: this.extractOperationName(document) ?? 'Query',
        operationType: 'query',
        document,
        variables,
      },
      options
    );
  }

  async mutate<TData, TVariables = Record<string, unknown>>(
    document: string,
    variables?: TVariables,
    options?: RequestOptions
  ): Promise<Result<TData, ClientError>> {
    return this.execute<TData, TVariables>(
      {
        operationName: this.extractOperationName(document) ?? 'Mutation',
        operationType: 'mutation',
        document,
        variables,
      },
      options
    );
  }

  use(middleware: Middleware): BgqlClient {
    this.middlewares.push(middleware);
    return this;
  }

  setHeaders(headers: Record<string, string>): void {
    this.headers = { ...this.headers, ...headers };
  }

  setAuthToken(token: string | null): void {
    if (token) {
      this.headers['Authorization'] = `Bearer ${token}`;
    } else {
      delete this.headers['Authorization'];
    }
  }

  private buildMiddlewareChain(): (
    operation: Operation<unknown, unknown>,
    options: RequestOptions
  ) => Promise<GraphQLResponse<unknown>> {
    type NextFn = (
      operation: Operation<unknown, unknown>,
      options: RequestOptions
    ) => Promise<GraphQLResponse<unknown>>;

    // The final handler that actually makes the request
    const finalHandler: NextFn = async (operation, options) => {
      return this.makeRequest(operation, options);
    };

    // Build the chain from the end to the beginning
    return this.middlewares.reduceRight<NextFn>(
      (next, middleware) => {
        return (operation, options) => middleware(operation, options, next);
      },
      finalHandler
    );
  }

  private async makeRequest(
    operation: Operation<unknown, unknown>,
    options: RequestOptions
  ): Promise<GraphQLResponse<unknown>> {
    const timeout = options.timeout ?? this.config.timeout ?? DEFAULT_CONFIG.timeout;
    const controller = new AbortController();

    // Link the provided signal to our controller
    if (options.signal) {
      options.signal.addEventListener('abort', () => controller.abort());
    }

    // Set up timeout
    const timeoutId = setTimeout(() => {
      controller.abort();
    }, timeout);

    try {
      const response = await this.fetchFn(this.config.url, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          Accept: 'application/json',
          ...this.headers,
          ...options.headers,
        },
        body: JSON.stringify({
          query: operation.document,
          variables: operation.variables,
          operationName: operation.operationName,
        }),
        credentials: this.config.credentials ?? DEFAULT_CONFIG.credentials,
        signal: controller.signal,
      });

      if (!response.ok) {
        throw new Error(`HTTP ${response.status}: ${response.statusText}`);
      }

      return (await response.json()) as GraphQLResponse<unknown>;
    } finally {
      clearTimeout(timeoutId);
    }
  }

  private processResponse<TData>(
    response: GraphQLResponse<unknown>
  ): Result<TData, ClientError> {
    // Check for GraphQL errors
    if (response.errors && response.errors.length > 0) {
      const firstError = response.errors[0];
      return err(
        graphqlExecutionError(firstError.message, {
          locations: firstError.locations,
          path: firstError.path,
          extensions: firstError.extensions,
        })
      );
    }

    // Return the data
    if (response.data !== undefined) {
      return ok(response.data as TData);
    }

    // No data and no errors - shouldn't happen
    return err(unknownError('No data returned from server'));
  }

  private handleError(error: unknown): ClientError {
    // Check for abort
    if (error instanceof DOMException && error.name === 'AbortError') {
      return abortError();
    }

    // Check for timeout (abort triggered by our timeout)
    if (error instanceof DOMException && error.name === 'TimeoutError') {
      return timeoutError(
        this.config.timeout ?? DEFAULT_CONFIG.timeout
      );
    }

    // Network errors
    if (error instanceof TypeError) {
      return networkError(error.message, {
        cause: error,
        retryable: true,
      });
    }

    // Generic errors
    if (error instanceof Error) {
      return unknownError(error.message, error);
    }

    return unknownError(String(error), error);
  }

  private extractOperationName(document: string): string | null {
    const match = document.match(
      /(?:query|mutation|subscription)\s+(\w+)/
    );
    return match?.[1] ?? null;
  }
}

// =============================================================================
// Middleware Helpers
// =============================================================================

/**
 * Creates a logging middleware.
 */
export function loggingMiddleware(
  logger: (message: string, data?: unknown) => void = console.log
): Middleware {
  return async (operation, options, next) => {
    const start = Date.now();
    logger(`[bgql] ${operation.operationType} ${operation.operationName}`, {
      variables: operation.variables,
    });

    try {
      const response = await next(operation, options);
      const duration = Date.now() - start;
      logger(`[bgql] ${operation.operationName} completed in ${duration}ms`, {
        hasErrors: !!response.errors?.length,
      });
      return response;
    } catch (error) {
      const duration = Date.now() - start;
      logger(`[bgql] ${operation.operationName} failed after ${duration}ms`, {
        error,
      });
      throw error;
    }
  };
}

/**
 * Creates a retry middleware.
 */
export function retryMiddleware(config: RetryConfig = {}): Middleware {
  const retryConfig = { ...DEFAULT_RETRY_CONFIG, ...config };

  return async (operation, options, next) => {
    let lastError: unknown;
    let delay = retryConfig.initialDelayMs;

    for (let attempt = 0; attempt <= retryConfig.maxRetries; attempt++) {
      try {
        return await next(operation, options);
      } catch (error) {
        lastError = error;

        // Check if we should retry
        if (
          attempt < retryConfig.maxRetries &&
          retryConfig.shouldRetry(error, attempt)
        ) {
          // Wait before retrying
          await sleep(delay);

          // Calculate next delay
          if (retryConfig.exponentialBackoff) {
            delay = Math.min(delay * 2, retryConfig.maxDelayMs);
          }
        } else {
          throw error;
        }
      }
    }

    throw lastError;
  };
}

/**
 * Creates a caching middleware.
 */
export function cachingMiddleware(
  cache: Map<string, { data: unknown; timestamp: number }>,
  ttlMs = 60000
): Middleware {
  return async (operation, options, next) => {
    // Only cache queries
    if (operation.operationType !== 'query') {
      return next(operation, options);
    }

    const key = JSON.stringify({
      name: operation.operationName,
      variables: operation.variables,
    });

    const cached = cache.get(key);
    if (cached && Date.now() - cached.timestamp < ttlMs) {
      return { data: cached.data } as GraphQLResponse<unknown>;
    }

    const response = await next(operation, options);

    if (response.data && !response.errors?.length) {
      cache.set(key, { data: response.data, timestamp: Date.now() });
    }

    return response;
  };
}

function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}
