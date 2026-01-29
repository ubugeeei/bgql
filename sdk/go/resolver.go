package sdk

import (
	"context"
	"sync"

	"golang.org/x/sync/singleflight"
)

// ResolverInfo contains metadata about the current resolution.
type ResolverInfo struct {
	FieldName  string
	ParentType string
	ReturnType string
	Path       []any
}

// ResolverFn is a typed resolver function.
type ResolverFn[TParent, TArgs, TResult any] func(
	ctx context.Context,
	parent TParent,
	args TArgs,
	info ResolverInfo,
) (TResult, error)

// RootResolverFn is a resolver without parent.
type RootResolverFn[TArgs, TResult any] func(
	ctx context.Context,
	args TArgs,
	info ResolverInfo,
) (TResult, error)

// SafeResolverFn returns a Result instead of error.
type SafeResolverFn[TParent, TArgs, TResult any] func(
	ctx context.Context,
	parent TParent,
	args TArgs,
	info ResolverInfo,
) Result[TResult]

// WrapResolver wraps a SafeResolverFn into a ResolverFn.
func WrapResolver[TParent, TArgs, TResult any](
	fn SafeResolverFn[TParent, TArgs, TResult],
) ResolverFn[TParent, TArgs, TResult] {
	return func(ctx context.Context, parent TParent, args TArgs, info ResolverInfo) (TResult, error) {
		result := fn(ctx, parent, args, info)
		if result.IsErr() {
			var zero TResult
			return zero, result.Error()
		}
		return result.Unwrap(), nil
	}
}

// DataLoader provides batching and caching for data fetching.
type DataLoader[K comparable, V any] struct {
	batchFn  func(ctx context.Context, keys []K) (map[K]V, error)
	cache    map[K]V
	mu       sync.RWMutex
	group    singleflight.Group
	maxBatch int
}

// DataLoaderConfig configures a DataLoader.
type DataLoaderConfig struct {
	MaxBatchSize int
	CacheEnabled bool
}

// NewDataLoader creates a new DataLoader.
func NewDataLoader[K comparable, V any](
	batchFn func(ctx context.Context, keys []K) (map[K]V, error),
	config *DataLoaderConfig,
) *DataLoader[K, V] {
	maxBatch := 100
	if config != nil && config.MaxBatchSize > 0 {
		maxBatch = config.MaxBatchSize
	}

	return &DataLoader[K, V]{
		batchFn:  batchFn,
		cache:    make(map[K]V),
		maxBatch: maxBatch,
	}
}

// Load loads a single value by key.
func (l *DataLoader[K, V]) Load(ctx context.Context, key K) (V, error) {
	l.mu.RLock()
	if value, ok := l.cache[key]; ok {
		l.mu.RUnlock()
		return value, nil
	}
	l.mu.RUnlock()

	// Use singleflight to deduplicate requests
	result, err, _ := l.group.Do(keyToString(key), func() (any, error) {
		results, err := l.batchFn(ctx, []K{key})
		if err != nil {
			return nil, err
		}

		l.mu.Lock()
		for k, v := range results {
			l.cache[k] = v
		}
		l.mu.Unlock()

		return results[key], nil
	})

	if err != nil {
		var zero V
		return zero, err
	}

	return result.(V), nil
}

// LoadMany loads multiple values by keys.
func (l *DataLoader[K, V]) LoadMany(ctx context.Context, keys []K) (map[K]V, error) {
	results := make(map[K]V)
	var missing []K

	l.mu.RLock()
	for _, key := range keys {
		if value, ok := l.cache[key]; ok {
			results[key] = value
		} else {
			missing = append(missing, key)
		}
	}
	l.mu.RUnlock()

	if len(missing) == 0 {
		return results, nil
	}

	loaded, err := l.batchFn(ctx, missing)
	if err != nil {
		return nil, err
	}

	l.mu.Lock()
	for k, v := range loaded {
		l.cache[k] = v
		results[k] = v
	}
	l.mu.Unlock()

	return results, nil
}

// Clear clears the cache.
func (l *DataLoader[K, V]) Clear() {
	l.mu.Lock()
	l.cache = make(map[K]V)
	l.mu.Unlock()
}

// Prime primes the cache with a value.
func (l *DataLoader[K, V]) Prime(key K, value V) {
	l.mu.Lock()
	l.cache[key] = value
	l.mu.Unlock()
}

func keyToString[K any](key K) string {
	return fmt.Sprintf("%v", key)
}

// FieldResolver wraps a typed resolver with error handling.
type FieldResolver[TParent, TArgs, TResult any] struct {
	resolve ResolverFn[TParent, TArgs, TResult]
}

// NewFieldResolver creates a new field resolver.
func NewFieldResolver[TParent, TArgs, TResult any](
	fn ResolverFn[TParent, TArgs, TResult],
) *FieldResolver[TParent, TArgs, TResult] {
	return &FieldResolver[TParent, TArgs, TResult]{resolve: fn}
}

// Resolve executes the resolver.
func (r *FieldResolver[TParent, TArgs, TResult]) Resolve(
	ctx context.Context,
	parent TParent,
	args TArgs,
	info ResolverInfo,
) Result[TResult] {
	result, err := r.resolve(ctx, parent, args, info)
	if err != nil {
		return Err[TResult](err)
	}
	return Ok(result)
}

// ResolverBuilder builds resolver maps with type safety.
type ResolverBuilder struct {
	resolvers map[string]map[string]any
}

// NewResolverBuilder creates a new resolver builder.
func NewResolverBuilder() *ResolverBuilder {
	return &ResolverBuilder{
		resolvers: make(map[string]map[string]any),
	}
}

// Register registers a resolver.
func Register[TParent, TArgs, TResult any](
	b *ResolverBuilder,
	typeName string,
	fieldName string,
	resolver ResolverFn[TParent, TArgs, TResult],
) *ResolverBuilder {
	if b.resolvers[typeName] == nil {
		b.resolvers[typeName] = make(map[string]any)
	}
	b.resolvers[typeName][fieldName] = resolver
	return b
}

// Query registers a root query resolver.
func Query[TArgs, TResult any](
	b *ResolverBuilder,
	fieldName string,
	resolver RootResolverFn[TArgs, TResult],
) *ResolverBuilder {
	return Register(b, "Query", fieldName, func(
		ctx context.Context,
		_ struct{},
		args TArgs,
		info ResolverInfo,
	) (TResult, error) {
		return resolver(ctx, args, info)
	})
}

// Mutation registers a root mutation resolver.
func Mutation[TArgs, TResult any](
	b *ResolverBuilder,
	fieldName string,
	resolver RootResolverFn[TArgs, TResult],
) *ResolverBuilder {
	return Register(b, "Mutation", fieldName, func(
		ctx context.Context,
		_ struct{},
		args TArgs,
		info ResolverInfo,
	) (TResult, error) {
		return resolver(ctx, args, info)
	})
}

// Build returns the resolver map.
func (b *ResolverBuilder) Build() map[string]map[string]any {
	return b.resolvers
}
