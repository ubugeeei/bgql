/**
 * Resolver utilities and type-safe resolver helpers.
 */

import type { BaseContext, ResolverFn, ResolveInfo } from './types';
import {
  isAuthenticated,
  requireAuth,
  requireRole,
  requirePermission,
} from './context';

// Re-export context helpers
export { isAuthenticated, requireAuth, requireRole, requirePermission };

/**
 * Creates a resolver with automatic authentication check.
 */
export function authenticated<TParent, TArgs, TContext extends BaseContext, TResult>(
  resolver: ResolverFn<TParent, TArgs, TContext, TResult>
): ResolverFn<
  TParent,
  TArgs,
  TContext,
  TResult | { __typename: 'UnauthorizedError'; message: string; code: string }
> {
  return (parent, args, context, info) => {
    return requireAuth(context, (authContext) =>
      resolver(parent, args, authContext as TContext, info)
    );
  };
}

/**
 * Creates a resolver with role-based authorization.
 */
export function withRole<TParent, TArgs, TContext extends BaseContext, TResult>(
  role: string,
  resolver: ResolverFn<TParent, TArgs, TContext, TResult>
): ResolverFn<
  TParent,
  TArgs,
  TContext,
  TResult | { __typename: 'UnauthorizedError' | 'ForbiddenError'; message: string; code: string }
> {
  return (parent, args, context, info) => {
    return requireRole(context, role, (authContext) =>
      resolver(parent, args, authContext as TContext, info)
    );
  };
}

/**
 * Creates a resolver with permission-based authorization.
 */
export function withPermission<TParent, TArgs, TContext extends BaseContext, TResult>(
  permission: string,
  resolver: ResolverFn<TParent, TArgs, TContext, TResult>
): ResolverFn<
  TParent,
  TArgs,
  TContext,
  TResult | { __typename: 'UnauthorizedError' | 'ForbiddenError'; message: string; code: string }
> {
  return (parent, args, context, info) => {
    return requirePermission(context, permission, (authContext) =>
      resolver(parent, args, authContext as TContext, info)
    );
  };
}

/**
 * Creates a resolver that handles null parent gracefully.
 */
export function nullable<TParent, TArgs, TContext, TResult>(
  resolver: ResolverFn<NonNullable<TParent>, TArgs, TContext, TResult>
): ResolverFn<TParent | null | undefined, TArgs, TContext, TResult | null> {
  return (parent, args, context, info) => {
    if (parent === null || parent === undefined) {
      return null;
    }
    return resolver(parent, args, context, info);
  };
}

/**
 * Combines multiple resolvers in sequence.
 */
export function pipe<TParent, TArgs, TContext, TResult>(
  ...resolvers: Array<
    (
      parent: unknown,
      args: unknown,
      context: unknown,
      info: ResolveInfo
    ) => unknown | Promise<unknown>
  >
): ResolverFn<TParent, TArgs, TContext, TResult> {
  return async (parent, args, context, info) => {
    let result: unknown = parent;
    for (const resolver of resolvers) {
      result = await resolver(result, args, context, info);
    }
    return result as TResult;
  };
}

/**
 * Creates a deferred resolver for @defer support.
 */
export function deferred<TParent, TArgs, TContext, TResult>(
  resolver: ResolverFn<TParent, TArgs, TContext, TResult>
): ResolverFn<TParent, TArgs, TContext, TResult> {
  return async (parent, args, context, info) => {
    return resolver(parent, args, context, info);
  };
}

/**
 * Creates a cached resolver.
 */
export function cached<TParent, TArgs, TContext extends { signal: AbortSignal }, TResult>(
  resolver: ResolverFn<TParent, TArgs, TContext, TResult>,
  options: {
    maxAge?: number;
    scope?: 'PUBLIC' | 'PRIVATE';
    cacheKey?: (parent: TParent, args: TArgs) => string;
  } = {}
): ResolverFn<TParent, TArgs, TContext, TResult> {
  const cache = new Map<string, { value: TResult; timestamp: number }>();
  const maxAge = options.maxAge ?? 60000;

  return async (parent, args, context, info) => {
    const key = options.cacheKey
      ? options.cacheKey(parent, args)
      : JSON.stringify({ parent, args });

    const cached = cache.get(key);
    if (cached && Date.now() - cached.timestamp < maxAge) {
      return cached.value;
    }

    const value = await resolver(parent, args, context, info);
    cache.set(key, { value, timestamp: Date.now() });

    // Set cache hint if available
    if (info.cacheControl) {
      info.cacheControl.setCacheHint({
        maxAge: Math.floor(maxAge / 1000),
        scope: options.scope,
      });
    }

    return value;
  };
}

/**
 * Creates a batched resolver using DataLoader pattern.
 */
export function batched<TParent, TArgs, TContext extends object, TResult, TKey = string>(
  keyFn: (parent: TParent, args: TArgs, context: TContext) => TKey,
  batchFn: (
    keys: ReadonlyArray<TKey>,
    context: TContext
  ) => Promise<Map<TKey, TResult>>
): ResolverFn<TParent, TArgs, TContext, TResult | null> {
  // Store pending batches by context (one batch per request)
  const pendingBatches = new WeakMap<
    object,
    {
      keys: TKey[];
      resolvers: Array<{
        key: TKey;
        resolve: (value: TResult | null) => void;
        reject: (error: Error) => void;
      }>;
      scheduled: boolean;
    }
  >();

  return (parent, args, context, _info) => {
    const key = keyFn(parent, args, context);

    return new Promise<TResult | null>((resolve, reject) => {
      // Get or create batch for this context
      let batch = pendingBatches.get(context);
      if (!batch) {
        batch = { keys: [], resolvers: [], scheduled: false };
        pendingBatches.set(context, batch);
      }

      batch.keys.push(key);
      batch.resolvers.push({ key, resolve, reject });

      // Schedule batch execution
      if (!batch.scheduled) {
        batch.scheduled = true;
        queueMicrotask(async () => {
          const currentBatch = pendingBatches.get(context);
          if (!currentBatch) return;

          pendingBatches.delete(context);

          try {
            const results = await batchFn(currentBatch.keys, context);
            for (const resolver of currentBatch.resolvers) {
              resolver.resolve(results.get(resolver.key) ?? null);
            }
          } catch (error) {
            for (const resolver of currentBatch.resolvers) {
              resolver.reject(error instanceof Error ? error : new Error(String(error)));
            }
          }
        });
      }
    });
  };
}

/**
 * Type helper for extracting parent type from resolver.
 */
export type ResolverParent<T> = T extends ResolverFn<infer P, unknown, unknown, unknown>
  ? P
  : never;

/**
 * Type helper for extracting args type from resolver.
 */
export type ResolverArgs<T> = T extends ResolverFn<unknown, infer A, unknown, unknown>
  ? A
  : never;

/**
 * Type helper for extracting context type from resolver.
 */
export type ResolverContext<T> = T extends ResolverFn<unknown, unknown, infer C, unknown>
  ? C
  : never;

/**
 * Type helper for extracting result type from resolver.
 */
export type ResolverResult<T> = T extends ResolverFn<unknown, unknown, unknown, infer R>
  ? R
  : never;
