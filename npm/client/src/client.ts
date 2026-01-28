/**
 * bgql Client Implementation
 *
 * Type-safe GraphQL client with Result-based error handling,
 * request deduplication, and automatic query batching.
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
  TypedDocumentNode,
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
 * Supports zero-config mode with just a URL string.
 *
 * @example
 * ```typescript
 * // Zero-config: just pass the URL
 * const client = createClient('http://localhost:4000/graphql');
 *
 * // Or with full configuration
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
export function createClient(config: string | ClientConfig): BgqlClient {
  const normalizedConfig = typeof config === 'string' ? { url: config } : config;
  return new BgqlClientImpl(normalizedConfig);
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
   * Executes a typed document node with full type inference.
   */
  executeTyped<TData, TVariables>(
    document: TypedDocumentNode<TData, TVariables>,
    variables: TVariables,
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
   * Executes a typed query document with full type inference.
   */
  queryTyped<TData, TVariables>(
    document: TypedDocumentNode<TData, TVariables>,
    variables: TVariables,
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
   * Executes a typed mutation document with full type inference.
   */
  mutateTyped<TData, TVariables>(
    document: TypedDocumentNode<TData, TVariables>,
    variables: TVariables,
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

  async executeTyped<TData, TVariables>(
    document: TypedDocumentNode<TData, TVariables>,
    variables: TVariables,
    options?: RequestOptions
  ): Promise<Result<TData, ClientError>> {
    return this.execute<TData, TVariables>(
      {
        operationName: document.__meta?.operationName ?? 'Operation',
        operationType: document.__meta?.operationType ?? 'query',
        document: document.__meta?.source ?? '',
        variables,
      },
      options
    );
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

  async queryTyped<TData, TVariables>(
    document: TypedDocumentNode<TData, TVariables>,
    variables: TVariables,
    options?: RequestOptions
  ): Promise<Result<TData, ClientError>> {
    return this.executeTyped(document, variables, options);
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

  async mutateTyped<TData, TVariables>(
    document: TypedDocumentNode<TData, TVariables>,
    variables: TVariables,
    options?: RequestOptions
  ): Promise<Result<TData, ClientError>> {
    return this.executeTyped(document, variables, options);
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

// =============================================================================
// Request Deduplication
// =============================================================================

/**
 * Creates a request deduplication middleware.
 * When the same query with the same variables is made multiple times simultaneously,
 * only one actual request is made and the result is shared.
 *
 * @example
 * ```typescript
 * const client = createClient('http://localhost:4000/graphql')
 *   .use(deduplicationMiddleware());
 *
 * // These will result in only ONE network request
 * const [result1, result2, result3] = await Promise.all([
 *   client.query(GetUserQuery, { id: '1' }),
 *   client.query(GetUserQuery, { id: '1' }),
 *   client.query(GetUserQuery, { id: '1' }),
 * ]);
 * ```
 */
export function deduplicationMiddleware(): Middleware {
  const inFlightRequests = new Map<string, Promise<GraphQLResponse<unknown>>>();

  return async (operation, options, next) => {
    // Only dedupe queries (mutations should never be deduped)
    if (operation.operationType !== 'query') {
      return next(operation, options);
    }

    const key = JSON.stringify({
      document: operation.document,
      variables: operation.variables,
      operationName: operation.operationName,
    });

    // Check if there's already an in-flight request for this key
    const existing = inFlightRequests.get(key);
    if (existing) {
      return existing;
    }

    // Create the request promise and store it
    const requestPromise = next(operation, options).finally(() => {
      // Remove from in-flight map when complete
      inFlightRequests.delete(key);
    });

    inFlightRequests.set(key, requestPromise);
    return requestPromise;
  };
}

// =============================================================================
// Query Batching
// =============================================================================

/**
 * Configuration for query batching.
 */
export interface BatchConfig {
  /**
   * Maximum number of operations to batch together.
   * @default 10
   */
  readonly maxBatchSize?: number;

  /**
   * Maximum time to wait before executing a batch (in ms).
   * @default 10
   */
  readonly batchInterval?: number;

  /**
   * Custom batch endpoint (if different from main endpoint).
   */
  readonly batchEndpoint?: string;
}

/**
 * Creates a query batching middleware.
 * Automatically batches multiple queries into a single HTTP request.
 *
 * Note: The server must support batched queries (array of operations).
 *
 * @example
 * ```typescript
 * const client = createClient('http://localhost:4000/graphql')
 *   .use(batchingMiddleware({ maxBatchSize: 10, batchInterval: 10 }));
 *
 * // These will be batched into a single HTTP request
 * const [users, posts, comments] = await Promise.all([
 *   client.query(GetUsersQuery),
 *   client.query(GetPostsQuery),
 *   client.query(GetCommentsQuery),
 * ]);
 * ```
 */
export function batchingMiddleware(config: BatchConfig = {}): Middleware {
  const maxBatchSize = config.maxBatchSize ?? 10;
  const batchInterval = config.batchInterval ?? 10;

  type BatchEntry = {
    operation: Operation<unknown, unknown>;
    options: RequestOptions;
    resolve: (response: GraphQLResponse<unknown>) => void;
    reject: (error: unknown) => void;
  };

  let batch: BatchEntry[] = [];
  let batchTimeout: ReturnType<typeof setTimeout> | null = null;
  let nextFn: ((op: Operation<unknown, unknown>, opts: RequestOptions) => Promise<GraphQLResponse<unknown>>) | null = null;

  const executeBatch = async () => {
    const currentBatch = batch;
    batch = [];
    batchTimeout = null;

    if (currentBatch.length === 0) return;

    // If only one operation, just execute it directly
    if (currentBatch.length === 1) {
      const entry = currentBatch[0];
      try {
        const response = await nextFn!(entry.operation, entry.options);
        entry.resolve(response);
      } catch (error) {
        entry.reject(error);
      }
      return;
    }

    // Execute batched request
    try {
      // Build batched request body
      const batchBody = currentBatch.map((entry) => ({
        query: entry.operation.document,
        variables: entry.operation.variables,
        operationName: entry.operation.operationName,
      }));

      // Get the first entry's options for headers, etc.
      const firstEntry = currentBatch[0];
      const response = await fetch(config.batchEndpoint ?? firstEntry.options.context?.url as string ?? '/graphql', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          Accept: 'application/json',
          ...(firstEntry.options.headers ?? {}),
        },
        body: JSON.stringify(batchBody),
      });

      const results = (await response.json()) as GraphQLResponse<unknown>[];

      // Resolve each entry with its corresponding result
      currentBatch.forEach((entry, index) => {
        if (Array.isArray(results) && results[index]) {
          entry.resolve(results[index]);
        } else {
          entry.reject(new Error('Invalid batch response'));
        }
      });
    } catch (error) {
      // Reject all entries on error
      currentBatch.forEach((entry) => entry.reject(error));
    }
  };

  return async (operation, options, next) => {
    // Store the next function for batch execution
    nextFn = next;

    // Only batch queries
    if (operation.operationType !== 'query') {
      return next(operation, options);
    }

    return new Promise((resolve, reject) => {
      batch.push({ operation, options, resolve, reject });

      // Execute immediately if batch is full
      if (batch.length >= maxBatchSize) {
        if (batchTimeout) {
          clearTimeout(batchTimeout);
          batchTimeout = null;
        }
        executeBatch();
      } else if (!batchTimeout) {
        // Schedule batch execution
        batchTimeout = setTimeout(executeBatch, batchInterval);
      }
    });
  };
}

// =============================================================================
// Normalized Cache
// =============================================================================

/**
 * Configuration for normalized cache.
 */
export interface NormalizedCacheConfig {
  /**
   * Function to extract the ID from an object.
   * @default (obj) => obj.id || obj._id
   */
  readonly getId?: (obj: Record<string, unknown>) => string | undefined;

  /**
   * Function to get the typename from an object.
   * @default (obj) => obj.__typename
   */
  readonly getTypename?: (obj: Record<string, unknown>) => string | undefined;

  /**
   * TTL for cache entries in milliseconds.
   * @default 300000 (5 minutes)
   */
  readonly ttlMs?: number;
}

/**
 * Creates a normalized cache for GraphQL data.
 * Normalizes entities by their type and ID for efficient updates and lookups.
 *
 * @example
 * ```typescript
 * const cache = createNormalizedCache();
 *
 * const client = createClient('http://localhost:4000/graphql')
 *   .use(normalizedCacheMiddleware(cache));
 *
 * // First request fetches from network
 * await client.query(GetUserQuery, { id: '1' });
 *
 * // Second request returns cached data
 * await client.query(GetUserQuery, { id: '1' });
 *
 * // After a mutation, you can update the cache
 * cache.update('User', '1', { name: 'New Name' });
 * ```
 */
export function createNormalizedCache(config: NormalizedCacheConfig = {}): NormalizedCache {
  const getId = config.getId ?? ((obj) => (obj.id ?? obj._id) as string | undefined);
  const getTypename = config.getTypename ?? ((obj) => obj.__typename as string | undefined);
  const ttlMs = config.ttlMs ?? 300000;

  const entities = new Map<string, Map<string, { data: Record<string, unknown>; timestamp: number }>>();
  const queryCache = new Map<string, { data: unknown; refs: string[]; timestamp: number }>();

  const getEntityKey = (typename: string, id: string) => `${typename}:${id}`;

  const normalizeObject = (obj: unknown, refs: string[]): unknown => {
    if (obj === null || typeof obj !== 'object') {
      return obj;
    }

    if (Array.isArray(obj)) {
      return obj.map((item) => normalizeObject(item, refs));
    }

    const record = obj as Record<string, unknown>;
    const typename = getTypename(record);
    const id = getId(record);

    // If this object has an ID and typename, normalize it
    if (typename && id) {
      const key = getEntityKey(typename, id);
      refs.push(key);

      // Store in entities
      if (!entities.has(typename)) {
        entities.set(typename, new Map());
      }

      const typeEntities = entities.get(typename)!;
      const normalized: Record<string, unknown> = {};

      for (const [k, v] of Object.entries(record)) {
        normalized[k] = normalizeObject(v, refs);
      }

      typeEntities.set(id, { data: normalized, timestamp: Date.now() });

      // Return a reference
      return { __ref: key };
    }

    // Normalize nested objects
    const result: Record<string, unknown> = {};
    for (const [k, v] of Object.entries(record)) {
      result[k] = normalizeObject(v, refs);
    }
    return result;
  };

  const denormalizeObject = (obj: unknown): unknown => {
    if (obj === null || typeof obj !== 'object') {
      return obj;
    }

    if (Array.isArray(obj)) {
      return obj.map(denormalizeObject);
    }

    const record = obj as Record<string, unknown>;

    // Check if this is a reference
    if ('__ref' in record && typeof record.__ref === 'string') {
      const [typename, id] = record.__ref.split(':');
      const entity = entities.get(typename)?.get(id);
      if (entity && Date.now() - entity.timestamp < ttlMs) {
        return denormalizeObject(entity.data);
      }
      return null;
    }

    // Denormalize nested objects
    const result: Record<string, unknown> = {};
    for (const [k, v] of Object.entries(record)) {
      result[k] = denormalizeObject(v);
    }
    return result;
  };

  return {
    read(queryKey: string) {
      const cached = queryCache.get(queryKey);
      if (!cached || Date.now() - cached.timestamp >= ttlMs) {
        return undefined;
      }
      return denormalizeObject(cached.data);
    },

    write(queryKey: string, data: unknown) {
      const refs: string[] = [];
      const normalized = normalizeObject(data, refs);
      queryCache.set(queryKey, { data: normalized, refs, timestamp: Date.now() });
    },

    update(typename: string, id: string, updates: Record<string, unknown>) {
      const typeEntities = entities.get(typename);
      if (typeEntities) {
        const entity = typeEntities.get(id);
        if (entity) {
          entity.data = { ...entity.data, ...updates };
          entity.timestamp = Date.now();
        }
      }
    },

    delete(typename: string, id: string) {
      const typeEntities = entities.get(typename);
      if (typeEntities) {
        typeEntities.delete(id);
      }
    },

    clear() {
      entities.clear();
      queryCache.clear();
    },

    getEntity(typename: string, id: string) {
      const entity = entities.get(typename)?.get(id);
      if (entity && Date.now() - entity.timestamp < ttlMs) {
        return denormalizeObject(entity.data) as Record<string, unknown>;
      }
      return undefined;
    },
  };
}

/**
 * Normalized cache interface.
 */
export interface NormalizedCache {
  /**
   * Reads cached data for a query.
   */
  read(queryKey: string): unknown | undefined;

  /**
   * Writes data to the cache.
   */
  write(queryKey: string, data: unknown): void;

  /**
   * Updates a cached entity.
   */
  update(typename: string, id: string, updates: Record<string, unknown>): void;

  /**
   * Deletes a cached entity.
   */
  delete(typename: string, id: string): void;

  /**
   * Clears the entire cache.
   */
  clear(): void;

  /**
   * Gets a single entity from the cache.
   */
  getEntity(typename: string, id: string): Record<string, unknown> | undefined;
}

/**
 * Creates a middleware that uses a normalized cache.
 *
 * @example
 * ```typescript
 * const cache = createNormalizedCache({ ttlMs: 60000 });
 * const client = createClient('http://localhost:4000/graphql')
 *   .use(normalizedCacheMiddleware(cache));
 * ```
 */
export function normalizedCacheMiddleware(cache: NormalizedCache): Middleware {
  return async (operation, options, next) => {
    // Only cache queries
    if (operation.operationType !== 'query') {
      const response = await next(operation, options);

      // After mutations, we might want to update the cache
      // This is a simple implementation - in production you'd want
      // to handle cache updates based on mutation results
      return response;
    }

    const queryKey = JSON.stringify({
      document: operation.document,
      variables: operation.variables,
      operationName: operation.operationName,
    });

    // Try to read from cache
    const cached = cache.read(queryKey);
    if (cached !== undefined) {
      return { data: cached } as GraphQLResponse<unknown>;
    }

    // Execute the request
    const response = await next(operation, options);

    // Write to cache if successful
    if (response.data && !response.errors?.length) {
      cache.write(queryKey, response.data);
    }

    return response;
  };
}

// =============================================================================
// gql Tagged Template Literal
// =============================================================================

/**
 * Tagged template literal for GraphQL queries with type inference support.
 * Returns a TypedDocumentNode that can be used with executeTyped/queryTyped/mutateTyped.
 *
 * @example
 * ```typescript
 * // Without type parameters (types need to be added manually or via codegen)
 * const query = gql`
 *   query GetUser($id: ID!) {
 *     user(id: $id) {
 *       id
 *       name
 *     }
 *   }
 * `;
 *
 * // With type parameters
 * const typedQuery = gql<{ user: User }, { id: string }>`
 *   query GetUser($id: ID!) {
 *     user(id: $id) {
 *       id
 *       name
 *     }
 *   }
 * `;
 * ```
 */
export function gql<TData = unknown, TVariables = Record<string, unknown>>(
  strings: TemplateStringsArray,
  ...values: unknown[]
): TypedDocumentNode<TData, TVariables> {
  // Combine template literal parts
  const source = strings.reduce((acc, str, i) => {
    return acc + str + (values[i] ?? '');
  }, '');

  // Extract operation info from the source
  const operationMatch = source.match(/^\s*(query|mutation|subscription)\s+(\w+)/);
  const operationType = (operationMatch?.[1] ?? 'query') as 'query' | 'mutation' | 'subscription';
  const operationName = operationMatch?.[2] ?? 'Operation';

  return {
    __meta: {
      operationName,
      operationType,
      source,
    },
  } as TypedDocumentNode<TData, TVariables>;
}

/**
 * Helper to create a typed document from a string.
 * Useful when you have the document string but want type safety.
 *
 * @example
 * ```typescript
 * const GetUserDocument = createTypedDocument<
 *   { user: { id: string; name: string } },
 *   { id: string }
 * >('GetUser', 'query', `
 *   query GetUser($id: ID!) {
 *     user(id: $id) { id name }
 *   }
 * `);
 * ```
 */
export function createTypedDocument<TData, TVariables = Record<string, unknown>>(
  operationName: string,
  operationType: 'query' | 'mutation' | 'subscription',
  source: string
): TypedDocumentNode<TData, TVariables> {
  return {
    __meta: {
      operationName,
      operationType,
      source,
    },
  } as TypedDocumentNode<TData, TVariables>;
}
