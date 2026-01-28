/**
 * Core types for the bgql server.
 */

import type { GraphQLResolveInfo } from 'graphql';

/**
 * Server configuration options.
 */
export interface ServerConfig {
  /**
   * Path to the bgql schema file.
   */
  readonly schema: string;

  /**
   * Resolver implementations.
   */
  readonly resolvers: Resolvers<BaseContext>;

  /**
   * Context factory function.
   */
  readonly context?: ContextFactory<BaseContext>;

  /**
   * DataLoader implementations.
   */
  readonly loaders?: LoaderFactory;

  /**
   * Server options.
   */
  readonly options?: ServerOptions;
}

/**
 * Server options.
 */
export interface ServerOptions {
  /**
   * Port to listen on.
   * @default 4000
   */
  readonly port?: number;

  /**
   * Host to bind to.
   * @default 'localhost'
   */
  readonly host?: string;

  /**
   * Enable introspection.
   * @default true in development, false in production
   */
  readonly introspection?: boolean;

  /**
   * Enable playground/graphiql.
   * @default true in development, false in production
   */
  readonly playground?: boolean;

  /**
   * Path for the playground endpoint.
   * @default '/playground'
   */
  readonly playgroundPath?: string;

  /**
   * Maximum query depth.
   */
  readonly maxDepth?: number;

  /**
   * Maximum query complexity.
   */
  readonly maxComplexity?: number;

  /**
   * Request timeout in milliseconds.
   * @default 30000
   */
  readonly timeout?: number;
}

/**
 * Base context interface that all contexts extend.
 */
export interface BaseContext {
  /**
   * Request information.
   */
  readonly request: RequestInfo;

  /**
   * Response helpers.
   */
  readonly response: ResponseHelpers;

  /**
   * Authentication state.
   */
  readonly auth: AuthInfo;

  /**
   * Abort signal for cancellation.
   */
  readonly signal: AbortSignal;
}

/**
 * Request information.
 */
export interface RequestInfo {
  readonly headers: Headers;
  readonly cookies: ReadonlyMap<string, string>;
  readonly ip: string;
  readonly method: string;
  readonly url: string;
}

/**
 * Response helpers for setting headers and cookies.
 */
export interface ResponseHelpers {
  setHeader(name: string, value: string): void;
  setCookie(name: string, value: string, options?: CookieOptions): void;
  deleteCookie(name: string): void;
}

/**
 * Cookie options.
 */
export interface CookieOptions {
  readonly maxAge?: number;
  readonly httpOnly?: boolean;
  readonly secure?: boolean;
  readonly sameSite?: 'Strict' | 'Lax' | 'None';
  readonly path?: string;
  readonly domain?: string;
}

/**
 * Authentication information.
 */
export interface AuthInfo {
  readonly user: AuthenticatedUser | null;
  readonly isAuthenticated: boolean;
  hasRole(role: string): boolean;
  hasScope(scope: string): boolean;
  hasPermission(permission: string): boolean;
}

/**
 * Authenticated user.
 */
export interface AuthenticatedUser {
  readonly id: string;
  readonly roles: ReadonlyArray<string>;
  readonly scopes: ReadonlyArray<string>;
  readonly permissions: ReadonlyArray<string>;
}

/**
 * Context factory function.
 */
export type ContextFactory<TContext extends BaseContext> = (
  req: IncomingRequest
) => Promise<TContext> | TContext;

/**
 * Incoming request (before context creation).
 */
export interface IncomingRequest {
  readonly headers: Headers;
  readonly method: string;
  readonly url: string;
  readonly body: unknown;
  readonly signal: AbortSignal;
}

/**
 * Loader factory function.
 */
export type LoaderFactory = () => Record<string, DataLoaderBatch<unknown, unknown>>;

/**
 * DataLoader batch function type.
 */
export type DataLoaderBatch<K, V> = (keys: ReadonlyArray<K>) => Promise<ReadonlyArray<V | Error>>;

/**
 * Resolver type definitions.
 * These are placeholders - actual types are generated from schema.
 */
export interface Resolvers<TContext extends BaseContext> {
  Query?: QueryResolvers<TContext>;
  Mutation?: MutationResolvers<TContext>;
  Subscription?: SubscriptionResolvers<TContext>;
  [typeName: string]: TypeResolvers<TContext> | undefined;
}

export type QueryResolvers<TContext> = Record<
  string,
  ResolverFn<unknown, unknown, TContext, unknown>
>;

export type MutationResolvers<TContext> = Record<
  string,
  ResolverFn<unknown, unknown, TContext, unknown>
>;

export type SubscriptionResolvers<TContext> = Record<
  string,
  SubscriptionResolverFn<unknown, unknown, TContext, unknown>
>;

export type TypeResolvers<TContext> = Record<
  string,
  ResolverFn<unknown, unknown, TContext, unknown>
>;

/**
 * Field resolver function type.
 */
export type ResolverFn<TParent, TArgs, TContext, TResult> = (
  parent: TParent,
  args: TArgs,
  context: TContext,
  info: ResolveInfo
) => TResult | Promise<TResult>;

/**
 * Subscription resolver function type.
 */
export interface SubscriptionResolverFn<TParent, TArgs, TContext, TResult> {
  subscribe: (
    parent: TParent,
    args: TArgs,
    context: TContext,
    info: ResolveInfo
  ) => AsyncIterator<TResult> | Promise<AsyncIterator<TResult>>;
  resolve?: (
    payload: TResult,
    args: TArgs,
    context: TContext,
    info: ResolveInfo
  ) => TResult | Promise<TResult>;
}

/**
 * Resolve info (extended from GraphQL).
 */
export interface ResolveInfo extends GraphQLResolveInfo {
  /**
   * Cache control hints.
   */
  cacheControl?: CacheControl;
}

/**
 * Cache control interface.
 */
export interface CacheControl {
  setCacheHint(hint: CacheHint): void;
}

/**
 * Cache hint.
 */
export interface CacheHint {
  maxAge?: number;
  scope?: 'PUBLIC' | 'PRIVATE';
}

// =============================================================================
// Result Types for Union Returns
// =============================================================================

/**
 * Base error interface for result unions.
 */
export interface BaseError {
  readonly __typename: string;
  readonly message: string;
  readonly code: string;
}

/**
 * Not found error.
 */
export interface NotFoundError extends BaseError {
  readonly __typename: 'NotFoundError';
  readonly resourceType: string;
  readonly resourceId: string;
}

/**
 * Validation error.
 */
export interface ValidationError extends BaseError {
  readonly __typename: 'ValidationError';
  readonly field: string;
  readonly constraint: string;
}

/**
 * Unauthorized error.
 */
export interface UnauthorizedError extends BaseError {
  readonly __typename: 'UnauthorizedError';
}

/**
 * Forbidden error.
 */
export interface ForbiddenError extends BaseError {
  readonly __typename: 'ForbiddenError';
  readonly requiredPermission?: string;
}

// =============================================================================
// Error Constructors
// =============================================================================

export function notFoundError(
  resourceType: string,
  resourceId: string
): NotFoundError {
  return {
    __typename: 'NotFoundError',
    message: `${resourceType} not found`,
    code: 'NOT_FOUND',
    resourceType,
    resourceId,
  };
}

export function validationError(
  field: string,
  constraint: string,
  message?: string
): ValidationError {
  return {
    __typename: 'ValidationError',
    message: message ?? `Validation failed for field '${field}': ${constraint}`,
    code: 'VALIDATION_ERROR',
    field,
    constraint,
  };
}

export function unauthorizedError(message = 'Authentication required'): UnauthorizedError {
  return {
    __typename: 'UnauthorizedError',
    message,
    code: 'UNAUTHORIZED',
  };
}

export function forbiddenError(
  message = 'Permission denied',
  requiredPermission?: string
): ForbiddenError {
  return {
    __typename: 'ForbiddenError',
    message,
    code: 'FORBIDDEN',
    requiredPermission,
  };
}
