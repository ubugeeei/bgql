/**
 * bgql Server Implementation
 *
 * A type-safe GraphQL server with built-in DataLoader, validation, and streaming support.
 */

import * as http from 'node:http';
import { graphql, buildSchema } from 'graphql';
import type { GraphQLSchema } from 'graphql';
import type {
  ServerConfig,
  ServerOptions,
  BaseContext,
  IncomingRequest,
  Resolvers,
} from './types';
import { createBaseContext } from './context';
import { formatError, BgqlServerError } from './errors';
import { DataLoader, createLoaderStore } from './dataloader';
import { generatePlaygroundHTML, acceptsHTML } from './playground';

/**
 * Check if we're in development mode.
 */
const isDevelopment = process.env.NODE_ENV !== 'production';

/**
 * Default server options with zero-config defaults.
 */
const DEFAULT_OPTIONS: Required<ServerOptions> = {
  port: 4000,
  host: '0.0.0.0', // Bind to all interfaces by default
  introspection: isDevelopment, // Auto-enable in development
  playground: isDevelopment, // Auto-enable in development
  playgroundPath: '/playground',
  maxDepth: 10,
  maxComplexity: 1000,
  timeout: 30000, // 30 second default timeout
};

/**
 * Default localhost origins for development CORS.
 */
const DEV_CORS_ORIGINS = [
  'http://localhost:3000',
  'http://localhost:4000',
  'http://localhost:5173',
  'http://localhost:5174',
  'http://localhost:8080',
  'http://127.0.0.1:3000',
  'http://127.0.0.1:4000',
  'http://127.0.0.1:5173',
  'http://127.0.0.1:5174',
  'http://127.0.0.1:8080',
];

/**
 * CORS configuration options.
 */
export interface CorsOptions {
  /**
   * Allowed origins. Use '*' for all origins or an array of specific origins.
   * @default '*'
   */
  readonly origin?: string | string[];

  /**
   * Allowed HTTP methods.
   * @default ['GET', 'POST', 'OPTIONS']
   */
  readonly methods?: string[];

  /**
   * Allowed headers.
   * @default ['Content-Type', 'Authorization']
   */
  readonly allowedHeaders?: string[];

  /**
   * Headers to expose to the client.
   */
  readonly exposedHeaders?: string[];

  /**
   * Allow credentials.
   * @default false
   */
  readonly credentials?: boolean;

  /**
   * Preflight cache duration in seconds.
   * @default 86400 (24 hours)
   */
  readonly maxAge?: number;
}

/**
 * Smart CORS defaults: allow all in production, localhost origins in development.
 */
const DEFAULT_CORS_OPTIONS: Required<CorsOptions> = {
  origin: isDevelopment ? DEV_CORS_ORIGINS : '*',
  methods: ['GET', 'POST', 'OPTIONS'],
  allowedHeaders: ['Content-Type', 'Authorization', 'X-Request-ID'],
  exposedHeaders: ['X-Request-ID'],
  credentials: isDevelopment, // Allow credentials in dev for local testing
  maxAge: 86400,
};

/**
 * Applies CORS headers to a response.
 */
function applyCorsHeaders(
  res: http.ServerResponse,
  req: http.IncomingMessage,
  options: CorsOptions = {}
): void {
  const opts = { ...DEFAULT_CORS_OPTIONS, ...options };
  const requestOrigin = req.headers.origin || '*';

  // Handle origin
  if (opts.origin === '*') {
    res.setHeader('Access-Control-Allow-Origin', '*');
  } else if (Array.isArray(opts.origin)) {
    if (opts.origin.includes(requestOrigin)) {
      res.setHeader('Access-Control-Allow-Origin', requestOrigin);
      res.setHeader('Vary', 'Origin');
    }
  } else {
    res.setHeader('Access-Control-Allow-Origin', opts.origin);
  }

  // Other CORS headers
  res.setHeader('Access-Control-Allow-Methods', opts.methods.join(', '));
  res.setHeader('Access-Control-Allow-Headers', opts.allowedHeaders.join(', '));

  if (opts.exposedHeaders.length > 0) {
    res.setHeader('Access-Control-Expose-Headers', opts.exposedHeaders.join(', '));
  }

  if (opts.credentials) {
    res.setHeader('Access-Control-Allow-Credentials', 'true');
  }

  res.setHeader('Access-Control-Max-Age', String(opts.maxAge));
}

/**
 * Creates a CORS middleware helper.
 */
export function corsMiddleware(options: CorsOptions = {}): (
  req: http.IncomingMessage,
  res: http.ServerResponse
) => boolean {
  return (req, res) => {
    applyCorsHeaders(res, req, options);

    // Handle preflight
    if (req.method === 'OPTIONS') {
      res.statusCode = 204;
      res.end();
      return true; // Request handled
    }

    return false; // Continue processing
  };
}

/**
 * Extended server configuration with CORS.
 */
export interface ExtendedServerConfig extends ServerConfig {
  /**
   * CORS configuration. Set to false to disable CORS.
   */
  readonly cors?: CorsOptions | false;
}

/**
 * bgql Server interface.
 */
export interface BgqlServer {
  /**
   * Starts the server.
   */
  listen(options?: { port?: number; host?: string }): Promise<ServerInfo>;

  /**
   * Stops the server gracefully.
   */
  stop(): Promise<void>;

  /**
   * Executes a GraphQL operation (for testing or programmatic use).
   */
  execute<TData = unknown>(options: {
    query: string;
    variables?: Record<string, unknown>;
    operationName?: string;
    context?: Partial<BaseContext>;
  }): Promise<ExecutionResult<TData>>;

  /**
   * Returns the underlying HTTP server (if started).
   */
  readonly httpServer: http.Server | null;
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
 * Parses the request body based on content type.
 */
async function parseRequestBody(
  req: http.IncomingMessage
): Promise<{ query?: string; variables?: Record<string, unknown>; operationName?: string }> {
  return new Promise((resolve, reject) => {
    let body = '';

    req.on('data', (chunk: Buffer) => {
      body += chunk.toString();
    });

    req.on('end', () => {
      const contentType = req.headers['content-type'] || '';

      try {
        if (contentType.includes('application/graphql')) {
          // application/graphql: body is the query string
          resolve({ query: body });
        } else if (contentType.includes('application/json') || body.startsWith('{')) {
          // application/json: parse as JSON
          const parsed = JSON.parse(body);
          resolve({
            query: parsed.query,
            variables: parsed.variables,
            operationName: parsed.operationName,
          });
        } else {
          // Try to parse as JSON anyway
          try {
            const parsed = JSON.parse(body);
            resolve({
              query: parsed.query,
              variables: parsed.variables,
              operationName: parsed.operationName,
            });
          } catch {
            // Treat as raw query
            resolve({ query: body });
          }
        }
      } catch (error) {
        reject(new Error('Invalid request body'));
      }
    });

    req.on('error', reject);
  });
}

/**
 * Sends a JSON response.
 */
function sendJson(
  res: http.ServerResponse,
  statusCode: number,
  data: unknown
): void {
  const json = JSON.stringify(data);
  res.statusCode = statusCode;
  res.setHeader('Content-Type', 'application/json; charset=utf-8');
  res.setHeader('Content-Length', Buffer.byteLength(json));
  res.end(json);
}

/**
 * Sends an HTML response.
 */
function sendHtml(
  res: http.ServerResponse,
  statusCode: number,
  html: string
): void {
  res.statusCode = statusCode;
  res.setHeader('Content-Type', 'text/html; charset=utf-8');
  res.setHeader('Content-Length', Buffer.byteLength(html));
  res.end(html);
}

/**
 * Builds a GraphQL schema from SDL string.
 */
function buildExecutableSchema(
  schemaSource: string,
  resolvers: Resolvers<BaseContext>
): GraphQLSchema {
  const schema = buildSchema(schemaSource);

  // Attach resolvers to the schema
  const typeMap = schema.getTypeMap();

  for (const [typeName, typeResolvers] of Object.entries(resolvers)) {
    const type = typeMap[typeName];
    if (!type || !('getFields' in type)) continue;

    const fields = type.getFields();
    for (const [fieldName, resolver] of Object.entries(typeResolvers || {})) {
      const field = fields[fieldName];
      if (field && typeof resolver === 'function') {
        (field as any).resolve = resolver;
      }
    }
  }

  return schema;
}

/**
 * Converts http.IncomingMessage headers to Headers object.
 */
function convertHeaders(incomingHeaders: http.IncomingHttpHeaders): Headers {
  const headers = new Headers();
  for (const [key, value] of Object.entries(incomingHeaders)) {
    if (value) {
      if (Array.isArray(value)) {
        for (const v of value) {
          headers.append(key, v);
        }
      } else {
        headers.set(key, value);
      }
    }
  }
  return headers;
}

/**
 * Creates a new bgql server.
 *
 * @example
 * ```typescript
 * const server = createServer({
 *   schema: `
 *     type Query {
 *       hello: String
 *       user(id: ID!): User
 *     }
 *     type User {
 *       id: ID!
 *       name: String!
 *     }
 *   `,
 *   resolvers: {
 *     Query: {
 *       hello: () => 'Hello, world!',
 *       user: async (_, { id }, ctx) => {
 *         return { id, name: 'John Doe' };
 *       },
 *     },
 *   },
 *   context: async (req) => ({
 *     ...createBaseContext(req),
 *     db: createDbClient(),
 *   }),
 * });
 *
 * const info = await server.listen({ port: 4000 });
 * console.log(`Server running at ${info.url}`);
 * ```
 */
export function createServer<TContext extends BaseContext = BaseContext>(
  config: ExtendedServerConfig
): BgqlServer {
  const options = { ...DEFAULT_OPTIONS, ...config.options };
  const corsOptions = config.cors !== false ? (config.cors || {}) : null;

  let httpServer: http.Server | null = null;
  let schema: GraphQLSchema | null = null;

  // Build schema (supports both SDL string and file path)
  const initSchema = (): GraphQLSchema => {
    if (schema) return schema;

    // For now, we expect schema to be an SDL string
    // In the future, this could load from a .bgql file
    if (typeof config.schema === 'string') {
      schema = buildExecutableSchema(config.schema, config.resolvers);
    } else {
      throw new Error('Schema must be a GraphQL SDL string');
    }

    return schema;
  };

  // Execute a GraphQL operation
  const executeOperation = async <TData = unknown>(
    query: string,
    variables?: Record<string, unknown>,
    operationName?: string,
    contextValue?: TContext
  ): Promise<ExecutionResult<TData>> => {
    const execSchema = initSchema();

    const result = await graphql({
      schema: execSchema,
      source: query,
      variableValues: variables,
      operationName,
      contextValue,
    });

    return {
      data: result.data as TData | undefined,
      errors: result.errors?.map((err) => formatError(err)),
    };
  };

  return {
    get httpServer() {
      return httpServer;
    },

    async listen(listenOptions) {
      const port = listenOptions?.port ?? options.port;
      const host = listenOptions?.host ?? options.host;

      // Initialize schema
      initSchema();

      // Create HTTP server
      httpServer = http.createServer(async (req, res) => {
        const url = new URL(req.url || '/', `http://${req.headers.host || 'localhost'}`);
        const pathname = url.pathname;

        // Apply CORS headers
        if (corsOptions) {
          const handled = corsMiddleware(corsOptions)(req, res);
          if (handled) return; // OPTIONS preflight was handled
        }

        // Health check endpoint
        if (pathname === '/health' && req.method === 'GET') {
          sendJson(res, 200, { status: 'ok' });
          return;
        }

        // GraphQL endpoint
        if (pathname === '/graphql') {
          // GET request - serve playground if enabled and HTML is accepted
          if (req.method === 'GET') {
            const headers = convertHeaders(req.headers);

            if (options.playground && acceptsHTML(headers)) {
              const playgroundHtml = generatePlaygroundHTML({
                endpoint: '/graphql',
                title: 'bgql Playground',
              });
              sendHtml(res, 200, playgroundHtml);
              return;
            }

            // GET without HTML accept - could be introspection via URL params
            const queryParam = url.searchParams.get('query');
            if (queryParam) {
              const variables = url.searchParams.get('variables');
              const operationName = url.searchParams.get('operationName');

              const controller = new AbortController();
              const timeoutId = setTimeout(() => controller.abort(), options.timeout);

              try {
                const incomingReq: IncomingRequest = {
                  headers,
                  method: 'GET',
                  url: req.url || '/graphql',
                  body: { query: queryParam },
                  signal: controller.signal,
                };

                const baseContext = createBaseContext(incomingReq);
                const context = config.context
                  ? await config.context(incomingReq)
                  : baseContext;

                const result = await executeOperation(
                  queryParam,
                  variables ? JSON.parse(variables) : undefined,
                  operationName || undefined,
                  context as TContext
                );

                sendJson(res, 200, result);
              } catch (error) {
                const message = error instanceof Error ? error.message : 'Unknown error';
                sendJson(res, 400, {
                  errors: [{ message, extensions: { code: 'BAD_REQUEST' } }],
                });
              } finally {
                clearTimeout(timeoutId);
              }
              return;
            }

            sendJson(res, 400, {
              errors: [{ message: 'Missing query parameter', extensions: { code: 'BAD_REQUEST' } }],
            });
            return;
          }

          // POST request - execute GraphQL operation
          if (req.method === 'POST') {
            const controller = new AbortController();
            const timeoutId = setTimeout(() => controller.abort(), options.timeout);

            try {
              const body = await parseRequestBody(req);

              if (!body.query) {
                sendJson(res, 400, {
                  errors: [{ message: 'Missing query in request body', extensions: { code: 'BAD_REQUEST' } }],
                });
                return;
              }

              const headers = convertHeaders(req.headers);
              const incomingReq: IncomingRequest = {
                headers,
                method: 'POST',
                url: req.url || '/graphql',
                body,
                signal: controller.signal,
              };

              const baseContext = createBaseContext(incomingReq);
              const context = config.context
                ? await config.context(incomingReq)
                : baseContext;

              const result = await executeOperation(
                body.query,
                body.variables,
                body.operationName,
                context as TContext
              );

              sendJson(res, 200, result);
            } catch (error) {
              if (error instanceof BgqlServerError) {
                sendJson(res, 400, {
                  errors: [error.toGraphQL()],
                });
              } else {
                const message = error instanceof Error ? error.message : 'Unknown error';
                sendJson(res, 500, {
                  errors: [{ message, extensions: { code: 'INTERNAL_SERVER_ERROR' } }],
                });
              }
            } finally {
              clearTimeout(timeoutId);
            }
            return;
          }

          // Method not allowed
          res.statusCode = 405;
          res.setHeader('Allow', 'GET, POST, OPTIONS');
          sendJson(res, 405, {
            errors: [{ message: 'Method not allowed', extensions: { code: 'METHOD_NOT_ALLOWED' } }],
          });
          return;
        }

        // Playground redirect
        if (options.playground && pathname === options.playgroundPath) {
          res.statusCode = 302;
          res.setHeader('Location', '/graphql');
          res.end();
          return;
        }

        // 404 for unknown routes
        sendJson(res, 404, {
          errors: [{ message: 'Not found', extensions: { code: 'NOT_FOUND' } }],
        });
      });

      return new Promise((resolve, reject) => {
        httpServer!.on('error', reject);

        httpServer!.listen(port, host, () => {
          console.log(`[bgql] Server running at http://${host}:${port}/graphql`);

          if (options.playground) {
            console.log(`[bgql] Playground available at http://${host}:${port}/graphql`);
          }

          resolve({
            url: `http://${host}:${port}`,
            port,
            host,
          });
        });
      });
    },

    async stop() {
      if (!httpServer) {
        return;
      }

      return new Promise<void>((resolve, reject) => {
        httpServer!.close((err) => {
          if (err) {
            reject(err);
          } else {
            console.log('[bgql] Server stopped');
            httpServer = null;
            resolve();
          }
        });
      });
    },

    async execute<TData = unknown>(executeOptions: {
      query: string;
      variables?: Record<string, unknown>;
      operationName?: string;
      context?: Partial<BaseContext>;
    }) {
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

        return executeOperation<TData>(
          executeOptions.query,
          executeOptions.variables,
          executeOptions.operationName,
          fullContext
        );
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
  config: ExtendedServerConfig
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

// =============================================================================
// Quick Start Helpers
// =============================================================================

/**
 * One-liner server start for quick prototyping and simple use cases.
 *
 * @example
 * ```typescript
 * import { serve } from '@bgql/server';
 *
 * // Start a GraphQL server with a single function call
 * const server = await serve(
 *   `type Query { hello: String! }`,
 *   { Query: { hello: () => 'Hello, world!' } }
 * );
 *
 * // Server is now running at http://0.0.0.0:4000/graphql
 * ```
 */
export async function serve(
  schema: string,
  resolvers: Resolvers<BaseContext>,
  options?: Partial<ServerOptions & { cors?: CorsOptions | false }>
): Promise<BgqlServer & ServerInfo> {
  const server = createServer({
    schema,
    resolvers,
    options,
    cors: options?.cors,
  });

  const info = await server.listen();

  // Return merged server + info for convenience
  return Object.assign(server, info);
}

/**
 * Quick start with automatic reload in development.
 * Uses sensible defaults optimized for development experience.
 *
 * @example
 * ```typescript
 * import { devServer } from '@bgql/server';
 *
 * devServer({
 *   schema: `type Query { hello: String! }`,
 *   resolvers: { Query: { hello: () => 'Hello, world!' } },
 * });
 * ```
 */
export async function devServer(config: ExtendedServerConfig): Promise<BgqlServer & ServerInfo> {
  const server = createServer({
    ...config,
    options: {
      playground: true,
      introspection: true,
      ...config.options,
    },
  });

  const info = await server.listen();

  // Log helpful development information
  console.log(`
[bgql] Development server started
  - GraphQL:    ${info.url}/graphql
  - Playground: ${info.url}/graphql (in browser)
  - Health:     ${info.url}/health
  `);

  return Object.assign(server, info);
}

// =============================================================================
// Type Inference Helpers for Resolvers
// =============================================================================

/**
 * Type helper to infer resolver types from a schema string.
 * This provides compile-time type checking for resolvers.
 *
 * Note: Full type inference requires code generation. This helper provides
 * a foundation for manual type definitions or use with bgql codegen.
 *
 * @example
 * ```typescript
 * // Define your schema types
 * interface User { id: string; name: string; email: string; }
 * interface Post { id: string; title: string; authorId: string; }
 *
 * // Create typed resolvers
 * const resolvers = defineResolvers<{
 *   Query: {
 *     user: { args: { id: string }; result: User | null };
 *     users: { args: {}; result: User[] };
 *   };
 *   Mutation: {
 *     createUser: { args: { name: string; email: string }; result: User };
 *   };
 *   User: {
 *     posts: { parent: User; args: {}; result: Post[] };
 *   };
 * }>()({
 *   Query: {
 *     user: (_, { id }) => db.users.find(u => u.id === id),
 *     users: () => db.users.all(),
 *   },
 *   Mutation: {
 *     createUser: (_, { name, email }) => db.users.create({ name, email }),
 *   },
 *   User: {
 *     posts: (user) => db.posts.findByAuthor(user.id),
 *   },
 * });
 * ```
 */
export function defineResolvers<TSchema extends ResolverTypeMap>() {
  return <T extends InferResolvers<TSchema>>(resolvers: T): T => resolvers;
}

/**
 * Type map for resolver definitions.
 * Each type maps field names to their argument and result types.
 */
export interface ResolverTypeMap {
  [typeName: string]: {
    [fieldName: string]: {
      parent?: unknown;
      args?: Record<string, unknown>;
      result: unknown;
    };
  };
}

/**
 * Infers resolver function types from a type map.
 */
export type InferResolvers<TMap extends ResolverTypeMap, TContext extends BaseContext = BaseContext> = {
  [TType in keyof TMap]?: {
    [TField in keyof TMap[TType]]?: (
      parent: TMap[TType][TField] extends { parent: infer P } ? P : unknown,
      args: TMap[TType][TField] extends { args: infer A } ? A : Record<string, unknown>,
      context: TContext,
      info: import('graphql').GraphQLResolveInfo
    ) => TMap[TType][TField]['result'] | Promise<TMap[TType][TField]['result']>;
  };
};

/**
 * Creates a typed resolver for a specific field.
 * Useful for defining resolvers outside the main resolver object.
 *
 * @example
 * ```typescript
 * const userResolver = createResolver<User | null, { id: string }, MyContext>(
 *   async (_, { id }, ctx) => {
 *     return ctx.db.users.findById(id);
 *   }
 * );
 * ```
 */
export function createResolver<TResult, TArgs = Record<string, unknown>, TContext extends BaseContext = BaseContext, TParent = unknown>(
  resolver: (parent: TParent, args: TArgs, context: TContext, info: import('graphql').GraphQLResolveInfo) => TResult | Promise<TResult>
): typeof resolver {
  return resolver;
}

/**
 * Creates a typed field resolver with parent type inference.
 * Useful for object type field resolvers.
 *
 * @example
 * ```typescript
 * const userPostsResolver = createFieldResolver<User, Post[], MyContext>(
 *   (user, _, ctx) => ctx.db.posts.findByAuthor(user.id)
 * );
 * ```
 */
export function createFieldResolver<TParent, TResult, TContext extends BaseContext = BaseContext, TArgs = Record<string, unknown>>(
  resolver: (parent: TParent, args: TArgs, context: TContext, info: import('graphql').GraphQLResolveInfo) => TResult | Promise<TResult>
): typeof resolver {
  return resolver;
}

// =============================================================================
// Server Configuration Helpers
// =============================================================================

/**
 * Creates a server configuration object with type inference.
 * Useful for separating configuration from server creation.
 *
 * @example
 * ```typescript
 * const config = defineServerConfig({
 *   schema: readFileSync('schema.graphql', 'utf-8'),
 *   resolvers,
 *   context: async (req) => ({ ...createBaseContext(req), db }),
 * });
 *
 * const server = createServer(config);
 * ```
 */
export function defineServerConfig<TContext extends BaseContext = BaseContext>(
  config: ExtendedServerConfig & { context?: (req: IncomingRequest) => Promise<TContext> | TContext }
): ExtendedServerConfig {
  return config;
}

/**
 * Merges multiple resolver objects into one.
 * Useful for organizing resolvers across multiple files.
 *
 * @example
 * ```typescript
 * const resolvers = mergeResolvers(
 *   queryResolvers,
 *   mutationResolvers,
 *   typeResolvers,
 * );
 * ```
 */
export function mergeResolvers<TContext extends BaseContext>(
  ...resolverMaps: Array<Partial<Resolvers<TContext>>>
): Resolvers<TContext> {
  const merged: Resolvers<TContext> = {};

  for (const resolvers of resolverMaps) {
    for (const [typeName, typeResolvers] of Object.entries(resolvers)) {
      if (!merged[typeName]) {
        merged[typeName] = {};
      }
      Object.assign(merged[typeName]!, typeResolvers);
    }
  }

  return merged;
}
