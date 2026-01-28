/**
 * bgql Server Implementation
 *
 * A type-safe GraphQL server with built-in DataLoader, validation, and streaming support.
 */

import type {
  ServerConfig,
  ServerOptions,
  BaseContext,
  IncomingRequest,
  Resolvers,
} from './types';
import { createBaseContext } from './context';
import { formatError } from './errors';
import { DataLoader, createLoaderStore } from './dataloader';

/**
 * Default server options.
 */
const DEFAULT_OPTIONS: Required<ServerOptions> = {
  port: 4000,
  host: 'localhost',
  introspection: process.env.NODE_ENV !== 'production',
  playground: process.env.NODE_ENV !== 'production',
  maxDepth: 10,
  maxComplexity: 1000,
  timeout: 30000,
};

/**
 * bgql Server interface.
 */
export interface BgqlServer {
  /**
   * Starts the server.
   */
  listen(options?: { port?: number; host?: string }): Promise<ServerInfo>;

  /**
   * Stops the server.
   */
  stop(): Promise<void>;

  /**
   * Executes a GraphQL operation (for testing).
   */
  execute<TData = unknown>(options: {
    query: string;
    variables?: Record<string, unknown>;
    operationName?: string;
    context?: Partial<BaseContext>;
  }): Promise<ExecutionResult<TData>>;
}

/**
 * Server info returned after starting.
 */
export interface ServerInfo {
  readonly url: string;
  readonly port: number;
  readonly host: string;
}

/**
 * Execution result.
 */
export interface ExecutionResult<TData = unknown> {
  readonly data?: TData;
  readonly errors?: ReadonlyArray<{
    message: string;
    extensions?: Record<string, unknown>;
  }>;
}

/**
 * Creates a new bgql server.
 *
 * @example
 * ```typescript
 * const server = createServer({
 *   schema: './schema.bgql',
 *   resolvers: {
 *     Query: {
 *       me: async (_, __, ctx) => ctx.auth.user,
 *       user: async (_, { id }, ctx) => {
 *         const user = await ctx.loaders.user.load(id);
 *         if (!user) {
 *           return notFoundError('User', id);
 *         }
 *         return user;
 *       },
 *     },
 *   },
 *   context: async (req) => ({
 *     ...createBaseContext(req),
 *     db: createDbClient(),
 *     loaders: createLoaders(),
 *   }),
 * });
 *
 * await server.listen({ port: 4000 });
 * ```
 */
export function createServer<TContext extends BaseContext = BaseContext>(
  config: ServerConfig
): BgqlServer {
  const options = { ...DEFAULT_OPTIONS, ...config.options };

  return {
    async listen(listenOptions) {
      const port = listenOptions?.port ?? options.port;
      const host = listenOptions?.host ?? options.host;

      // Note: Actual HTTP server implementation would depend on the runtime
      // (Node.js, Bun, Deno, etc.)
      // This is a placeholder that shows the interface

      console.log(`[bgql] Server starting on http://${host}:${port}`);

      if (options.playground) {
        console.log(`[bgql] Playground available at http://${host}:${port}/graphql`);
      }

      return {
        url: `http://${host}:${port}`,
        port,
        host,
      };
    },

    async stop() {
      console.log('[bgql] Server stopped');
    },

    async execute<TData = unknown>(executeOptions) {
      const controller = new AbortController();
      const timeoutId = setTimeout(() => controller.abort(), options.timeout);

      try {
        // Create context
        const req: IncomingRequest = {
          headers: new Headers(),
          method: 'POST',
          url: '/graphql',
          body: {
            query: executeOptions.query,
            variables: executeOptions.variables,
            operationName: executeOptions.operationName,
          },
          signal: controller.signal,
        };

        const baseContext = createBaseContext(req);
        const context = config.context
          ? await config.context(req)
          : baseContext;

        // Merge provided context overrides
        const fullContext = {
          ...context,
          ...executeOptions.context,
        } as TContext;

        // Execute the query
        // Note: This would use the actual GraphQL execution engine
        // For now, return a placeholder

        return {
          data: undefined as TData | undefined,
          errors: undefined,
        };
      } finally {
        clearTimeout(timeoutId);
      }
    },
  };
}

/**
 * Creates a test client for a server.
 */
export function createTestClient<TContext extends BaseContext = BaseContext>(
  config: ServerConfig
): {
  query<TData = unknown, TVariables = Record<string, unknown>>(options: {
    query: string;
    variables?: TVariables;
    context?: Partial<TContext>;
  }): Promise<ExecutionResult<TData>>;
  mutate<TData = unknown, TVariables = Record<string, unknown>>(options: {
    mutation: string;
    variables?: TVariables;
    context?: Partial<TContext>;
  }): Promise<ExecutionResult<TData>>;
} {
  const server = createServer(config);

  return {
    async query<TData = unknown, TVariables = Record<string, unknown>>(options: {
      query: string;
      variables?: TVariables;
      context?: Partial<TContext>;
    }) {
      return server.execute<TData>({
        query: options.query,
        variables: options.variables as Record<string, unknown>,
        context: options.context as Partial<BaseContext>,
      });
    },

    async mutate<TData = unknown, TVariables = Record<string, unknown>>(options: {
      mutation: string;
      variables?: TVariables;
      context?: Partial<TContext>;
    }) {
      return server.execute<TData>({
        query: options.mutation,
        variables: options.variables as Record<string, unknown>,
        context: options.context as Partial<BaseContext>,
      });
    },
  };
}

/**
 * Middleware function type for request processing.
 */
export type ServerMiddleware<TContext extends BaseContext = BaseContext> = (
  context: TContext,
  next: () => Promise<ExecutionResult>
) => Promise<ExecutionResult>;

/**
 * Creates a middleware stack.
 */
export function createMiddlewareStack<TContext extends BaseContext>(
  ...middlewares: ServerMiddleware<TContext>[]
): ServerMiddleware<TContext> {
  return async (context, finalNext) => {
    let index = 0;

    const next = async (): Promise<ExecutionResult> => {
      if (index >= middlewares.length) {
        return finalNext();
      }
      const middleware = middlewares[index++];
      return middleware(context, next);
    };

    return next();
  };
}

/**
 * Built-in middleware: Logging.
 */
export function loggingMiddleware<TContext extends BaseContext>(
  logger: (message: string, data?: unknown) => void = console.log
): ServerMiddleware<TContext> {
  return async (context, next) => {
    const start = Date.now();
    logger('[bgql] Request started', {
      method: context.request.method,
      url: context.request.url,
    });

    try {
      const result = await next();
      const duration = Date.now() - start;
      logger('[bgql] Request completed', { duration, hasErrors: !!result.errors });
      return result;
    } catch (error) {
      const duration = Date.now() - start;
      logger('[bgql] Request failed', { duration, error });
      throw error;
    }
  };
}

/**
 * Built-in middleware: Rate limiting.
 */
export function rateLimitMiddleware<TContext extends BaseContext>(options: {
  windowMs: number;
  maxRequests: number;
  keyFn?: (context: TContext) => string;
}): ServerMiddleware<TContext> {
  const requests = new Map<string, { count: number; resetTime: number }>();

  return async (context, next) => {
    const key = options.keyFn?.(context) ?? context.request.ip;
    const now = Date.now();

    let entry = requests.get(key);
    if (!entry || now > entry.resetTime) {
      entry = { count: 0, resetTime: now + options.windowMs };
      requests.set(key, entry);
    }

    entry.count++;

    if (entry.count > options.maxRequests) {
      return {
        errors: [
          {
            message: 'Rate limit exceeded',
            extensions: {
              code: 'RATE_LIMITED',
              retryAfter: entry.resetTime - now,
            },
          },
        ],
      };
    }

    return next();
  };
}
