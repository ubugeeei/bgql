/**
 * Strongly typed GraphQL server utilities.
 *
 * Provides type-safe resolver definitions and context handling.
 */

import { type SdkError, ErrorCode, sdkError } from "./error";
import { type Result, ok, err, isOk } from "./result";
import {
  TypedContext,
  type ContextKey,
  createContext,
  buildContext,
} from "./context";

/**
 * Resolver info - metadata about the current resolution.
 */
export interface ResolverInfo {
  readonly fieldName: string;
  readonly parentType: string;
  readonly returnType: string;
  readonly path: readonly (string | number)[];
}

/**
 * Typed resolver function.
 */
export type ResolverFn<
  TParent,
  TArgs,
  TContext extends TypedContext,
  TResult
> = (
  parent: TParent,
  args: TArgs,
  context: TContext,
  info: ResolverInfo
) => TResult | Promise<TResult>;

/**
 * Typed resolver that returns Result.
 */
export type SafeResolverFn<
  TParent,
  TArgs,
  TContext extends TypedContext,
  TResult
> = (
  parent: TParent,
  args: TArgs,
  context: TContext,
  info: ResolverInfo
) => Result<TResult> | Promise<Result<TResult>>;

/**
 * Root resolver (no parent).
 */
export type RootResolverFn<TArgs, TContext extends TypedContext, TResult> = (
  args: TArgs,
  context: TContext,
  info: ResolverInfo
) => TResult | Promise<TResult>;

/**
 * Resolver map type for a single type.
 */
export type TypeResolvers<
  TParent,
  TContext extends TypedContext,
  TFields extends Record<string, unknown>
> = {
  [K in keyof TFields]?: ResolverFn<
    TParent,
    TFields[K] extends { args: infer A } ? A : Record<string, never>,
    TContext,
    TFields[K] extends { result: infer R } ? R : TFields[K]
  >;
};

/**
 * Root operation resolvers.
 */
export interface RootResolvers<TContext extends TypedContext = TypedContext> {
  Query?: Record<string, RootResolverFn<unknown, TContext, unknown>>;
  Mutation?: Record<string, RootResolverFn<unknown, TContext, unknown>>;
  Subscription?: Record<
    string,
    {
      subscribe: RootResolverFn<unknown, TContext, AsyncIterable<unknown>>;
      resolve?: (payload: unknown) => unknown;
    }
  >;
}

/**
 * Custom type resolvers map.
 */
export type CustomTypeResolvers<TContext extends TypedContext = TypedContext> = Record<
  string,
  Record<string, ResolverFn<unknown, unknown, TContext, unknown>> | undefined
>;

/**
 * Full resolver map.
 */
export type Resolvers<TContext extends TypedContext = TypedContext> =
  RootResolvers<TContext> & CustomTypeResolvers<TContext>;

/**
 * Creates a type-safe resolver.
 */
export function resolver<TParent, TArgs, TContext extends TypedContext, TResult>(
  fn: ResolverFn<TParent, TArgs, TContext, TResult>
): ResolverFn<TParent, TArgs, TContext, TResult> {
  return fn;
}

/**
 * Creates a type-safe root resolver.
 */
export function rootResolver<TArgs, TContext extends TypedContext, TResult>(
  fn: RootResolverFn<TArgs, TContext, TResult>
): RootResolverFn<TArgs, TContext, TResult> {
  return fn;
}

/**
 * Wraps a resolver with error handling that returns Result.
 */
export function safeResolver<
  TParent,
  TArgs,
  TContext extends TypedContext,
  TResult
>(
  fn: SafeResolverFn<TParent, TArgs, TContext, TResult>
): ResolverFn<TParent, TArgs, TContext, TResult> {
  return async (parent, args, context, info) => {
    const result = await fn(parent, args, context, info);
    if (isOk(result)) {
      return result.value;
    }
    throw result.error;
  };
}

/**
 * Context extractor type.
 */
export type ContextExtractor<T> = (context: TypedContext) => T;

/**
 * Creates a context extractor.
 */
export function extractor<T>(key: ContextKey<T>): ContextExtractor<T> {
  return (context) => {
    const value = context.get(key);
    if (value === undefined) {
      throw new Error(`Required context key not found: ${key.toString()}`);
    }
    return value;
  };
}

/**
 * Creates an optional context extractor.
 */
export function optionalExtractor<T>(
  key: ContextKey<T>
): ContextExtractor<T | undefined> {
  return (context) => context.get(key);
}

/**
 * Combines multiple extractors into a tuple.
 */
export function extractAll<T extends readonly unknown[]>(
  ...extractors: { [K in keyof T]: ContextExtractor<T[K]> }
): ContextExtractor<T> {
  return (context) => extractors.map((e) => e(context)) as unknown as T;
}

/**
 * DataLoader interface.
 */
export interface DataLoader<K, V> {
  load(key: K): Promise<V | undefined>;
  loadMany(keys: readonly K[]): Promise<ReadonlyMap<K, V>>;
  clear(): void;
  prime(key: K, value: V): void;
}

/**
 * Batch load function type.
 */
export type BatchLoadFn<K, V> = (keys: readonly K[]) => Promise<ReadonlyMap<K, V>>;

/**
 * Creates a DataLoader.
 */
export function createDataLoader<K, V>(
  batchFn: BatchLoadFn<K, V>,
  options?: { maxBatchSize?: number; cacheEnabled?: boolean }
): DataLoader<K, V> {
  const cache = new Map<K, V>();
  const pending = new Map<K, Promise<V | undefined>>();
  let batchKeys: K[] = [];
  let batchPromise: Promise<void> | null = null;

  const scheduleBatch = () => {
    if (batchPromise) return;

    batchPromise = Promise.resolve().then(async () => {
      const keys = batchKeys;
      batchKeys = [];
      batchPromise = null;

      if (keys.length === 0) return;

      const results = await batchFn(keys);
      for (const key of keys) {
        const value = results.get(key);
        if (value !== undefined && options?.cacheEnabled !== false) {
          cache.set(key, value);
        }
      }
    });
  };

  return {
    async load(key) {
      if (cache.has(key)) {
        return cache.get(key);
      }

      if (pending.has(key)) {
        return pending.get(key);
      }

      const promise = (async () => {
        batchKeys.push(key);
        scheduleBatch();
        await batchPromise;
        return cache.get(key);
      })();

      pending.set(key, promise);
      const result = await promise;
      pending.delete(key);
      return result;
    },

    async loadMany(keys) {
      const results = new Map<K, V>();
      const missing: K[] = [];

      for (const key of keys) {
        if (cache.has(key)) {
          results.set(key, cache.get(key)!);
        } else {
          missing.push(key);
        }
      }

      if (missing.length > 0) {
        const loaded = await batchFn(missing);
        for (const [key, value] of loaded) {
          results.set(key, value);
          if (options?.cacheEnabled !== false) {
            cache.set(key, value);
          }
        }
      }

      return results;
    },

    clear() {
      cache.clear();
    },

    prime(key, value) {
      cache.set(key, value);
    },
  };
}

// Re-exports
export { TypedContext, contextKey, createContext, buildContext } from "./context";
export type { ContextKey } from "./context";
export { type SdkError, ErrorCode, sdkError } from "./error";
export { type Result, ok, err, isOk, isErr } from "./result";
