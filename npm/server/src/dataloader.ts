/**
 * DataLoader implementation for N+1 prevention.
 *
 * Based on the DataLoader pattern but with bgql-specific enhancements.
 */

/**
 * Batch loading function type.
 */
export type BatchLoadFn<K, V> = (
  keys: ReadonlyArray<K>
) => Promise<ReadonlyArray<V | Error>>;

/**
 * DataLoader options.
 */
export interface DataLoaderOptions<K, V, C = K> {
  /**
   * Whether to batch loads.
   * @default true
   */
  readonly batch?: boolean;

  /**
   * Maximum batch size.
   * @default Infinity
   */
  readonly maxBatchSize?: number;

  /**
   * Whether to cache results.
   * @default true
   */
  readonly cache?: boolean;

  /**
   * Custom cache key function.
   */
  readonly cacheKeyFn?: (key: K) => C;

  /**
   * Custom cache map factory.
   */
  readonly cacheMap?: CacheMap<C, Promise<V>> | null;

  /**
   * Batch scheduling function.
   */
  readonly batchScheduleFn?: (callback: () => void) => void;

  /**
   * Name for debugging/tracing.
   */
  readonly name?: string;
}

/**
 * Cache map interface.
 */
export interface CacheMap<K, V> {
  get(key: K): V | undefined;
  set(key: K, value: V): void;
  delete(key: K): void;
  clear(): void;
}

/**
 * DataLoader class for batching and caching.
 */
export class DataLoader<K, V, C = K> {
  private readonly batchLoadFn: BatchLoadFn<K, V>;
  private readonly options: Required<
    Pick<
      DataLoaderOptions<K, V, C>,
      'batch' | 'maxBatchSize' | 'cache' | 'cacheKeyFn' | 'batchScheduleFn'
    >
  > & { cacheMap: CacheMap<C, Promise<V>> | null; name?: string };
  private batch: { keys: K[]; callbacks: Array<{ resolve: (value: V) => void; reject: (error: Error) => void }> } | null = null;

  constructor(
    batchLoadFn: BatchLoadFn<K, V>,
    options?: DataLoaderOptions<K, V, C>
  ) {
    this.batchLoadFn = batchLoadFn;
    this.options = {
      batch: options?.batch ?? true,
      maxBatchSize: options?.maxBatchSize ?? Infinity,
      cache: options?.cache ?? true,
      cacheKeyFn: options?.cacheKeyFn ?? ((key: K) => key as unknown as C),
      cacheMap: options?.cacheMap === null ? null : (options?.cacheMap ?? new Map()),
      batchScheduleFn: options?.batchScheduleFn ?? ((cb) => queueMicrotask(cb)),
      name: options?.name,
    };
  }

  /**
   * Loads a single value by key.
   */
  async load(key: K): Promise<V> {
    const cacheKey = this.options.cacheKeyFn(key);

    // Check cache
    if (this.options.cache && this.options.cacheMap) {
      const cached = this.options.cacheMap.get(cacheKey);
      if (cached !== undefined) {
        return cached;
      }
    }

    // Create promise for this load
    const promise = new Promise<V>((resolve, reject) => {
      // Add to current batch
      if (this.options.batch) {
        this.addToBatch(key, resolve, reject);
      } else {
        // Execute immediately
        this.batchLoadFn([key])
          .then((values) => {
            const value = values[0];
            if (value instanceof Error) {
              reject(value);
            } else {
              resolve(value);
            }
          })
          .catch(reject);
      }
    });

    // Cache the promise
    if (this.options.cache && this.options.cacheMap) {
      this.options.cacheMap.set(cacheKey, promise);
    }

    return promise;
  }

  /**
   * Loads multiple values by keys.
   */
  async loadMany(keys: ReadonlyArray<K>): Promise<Array<V | Error>> {
    return Promise.all(
      keys.map((key) =>
        this.load(key).catch((error: Error) => error)
      )
    );
  }

  /**
   * Clears a single key from the cache.
   */
  clear(key: K): this {
    if (this.options.cacheMap) {
      const cacheKey = this.options.cacheKeyFn(key);
      this.options.cacheMap.delete(cacheKey);
    }
    return this;
  }

  /**
   * Clears all keys from the cache.
   */
  clearAll(): this {
    if (this.options.cacheMap) {
      this.options.cacheMap.clear();
    }
    return this;
  }

  /**
   * Primes the cache with a value.
   */
  prime(key: K, value: V | Error): this {
    if (this.options.cacheMap) {
      const cacheKey = this.options.cacheKeyFn(key);
      if (this.options.cacheMap.get(cacheKey) === undefined) {
        this.options.cacheMap.set(
          cacheKey,
          value instanceof Error ? Promise.reject(value) : Promise.resolve(value)
        );
      }
    }
    return this;
  }

  /**
   * Gets the name of this DataLoader.
   */
  get name(): string | undefined {
    return this.options.name;
  }

  private addToBatch(
    key: K,
    resolve: (value: V) => void,
    reject: (error: Error) => void
  ): void {
    // Create new batch if needed
    if (!this.batch) {
      this.batch = { keys: [], callbacks: [] };
      this.options.batchScheduleFn(() => this.dispatchBatch());
    }

    this.batch.keys.push(key);
    this.batch.callbacks.push({ resolve, reject });

    // Dispatch if max batch size reached
    if (this.batch.keys.length >= this.options.maxBatchSize) {
      this.dispatchBatch();
    }
  }

  private dispatchBatch(): void {
    const batch = this.batch;
    if (!batch) return;

    this.batch = null;
    const { keys, callbacks } = batch;

    // Execute batch load
    this.batchLoadFn(keys)
      .then((values) => {
        if (values.length !== keys.length) {
          throw new Error(
            `DataLoader batch function returned ${values.length} results for ${keys.length} keys`
          );
        }

        for (let i = 0; i < values.length; i++) {
          const value = values[i];
          const callback = callbacks[i];

          if (value instanceof Error) {
            callback.reject(value);
          } else {
            callback.resolve(value);
          }
        }
      })
      .catch((error: Error) => {
        for (const callback of callbacks) {
          callback.reject(error);
        }
      });
  }
}

/**
 * Creates a DataLoader from a simple key-value lookup function.
 */
export function createLoader<K extends string | number, V>(
  loadFn: (keys: ReadonlyArray<K>) => Promise<ReadonlyMap<K, V>>,
  options?: DataLoaderOptions<K, V | null, K>
): DataLoader<K, V | null, K> {
  return new DataLoader(async (keys) => {
    const result = await loadFn(keys);
    return keys.map((key) => result.get(key) ?? null);
  }, options);
}

/**
 * Creates a DataLoader for loading related entities.
 */
export function createRelationLoader<K extends string | number, V>(
  loadFn: (keys: ReadonlyArray<K>) => Promise<ReadonlyMap<K, V[]>>,
  options?: DataLoaderOptions<K, V[], K>
): DataLoader<K, V[], K> {
  return new DataLoader(async (keys) => {
    const result = await loadFn(keys);
    return keys.map((key) => result.get(key) ?? []);
  }, options);
}

/**
 * Creates a simple caching store for DataLoaders.
 */
export function createLoaderStore(): {
  getLoader<K, V, C = K>(
    name: string,
    factory: () => DataLoader<K, V, C>
  ): DataLoader<K, V, C>;
  clearAll(): void;
} {
  const loaders = new Map<string, DataLoader<unknown, unknown, unknown>>();

  return {
    getLoader<K, V, C = K>(
      name: string,
      factory: () => DataLoader<K, V, C>
    ): DataLoader<K, V, C> {
      let loader = loaders.get(name);
      if (!loader) {
        loader = factory() as DataLoader<unknown, unknown, unknown>;
        loaders.set(name, loader);
      }
      return loader as DataLoader<K, V, C>;
    },

    clearAll(): void {
      for (const loader of loaders.values()) {
        loader.clearAll();
      }
      loaders.clear();
    },
  };
}
