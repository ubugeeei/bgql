package sdk

import (
	"context"
	"net/http"
	"sync"
)

// ContextKey is a typed key for context values.
type ContextKey[T any] struct {
	name string
}

// NewContextKey creates a new typed context key.
func NewContextKey[T any](name string) ContextKey[T] {
	return ContextKey[T]{name: name}
}

// String returns the key name.
func (k ContextKey[T]) String() string {
	return k.name
}

// Set stores a value in the context.
func (k ContextKey[T]) Set(ctx context.Context, value T) context.Context {
	return context.WithValue(ctx, k, value)
}

// Get retrieves a value from the context.
func (k ContextKey[T]) Get(ctx context.Context) (T, bool) {
	value, ok := ctx.Value(k).(T)
	return value, ok
}

// MustGet retrieves a value or panics.
func (k ContextKey[T]) MustGet(ctx context.Context) T {
	value, ok := k.Get(ctx)
	if !ok {
		panic("required context key not found: " + k.name)
	}
	return value
}

// GetOrDefault retrieves a value or returns a default.
func (k ContextKey[T]) GetOrDefault(ctx context.Context, defaultValue T) T {
	value, ok := k.Get(ctx)
	if !ok {
		return defaultValue
	}
	return value
}

// Common context keys
var (
	CurrentUserID   = NewContextKey[string]("CurrentUserID")
	UserRoles       = NewContextKey[[]string]("UserRoles")
	RequestID       = NewContextKey[string]("RequestID")
	RequestHeaders  = NewContextKey[http.Header]("RequestHeaders")
)

// RolesHelper provides role checking utilities.
type RolesHelper struct {
	roles []string
}

// NewRolesHelper creates a new roles helper.
func NewRolesHelper(roles []string) *RolesHelper {
	return &RolesHelper{roles: roles}
}

// Has checks if a role exists.
func (h *RolesHelper) Has(role string) bool {
	for _, r := range h.roles {
		if r == role {
			return true
		}
	}
	return false
}

// HasAny checks if any of the roles exist.
func (h *RolesHelper) HasAny(roles ...string) bool {
	for _, role := range roles {
		if h.Has(role) {
			return true
		}
	}
	return false
}

// HasAll checks if all roles exist.
func (h *RolesHelper) HasAll(roles ...string) bool {
	for _, role := range roles {
		if !h.Has(role) {
			return false
		}
	}
	return true
}

// Roles returns all roles.
func (h *RolesHelper) Roles() []string {
	return h.roles
}

// GetRolesHelper extracts roles from context and creates a helper.
func GetRolesHelper(ctx context.Context) *RolesHelper {
	roles, ok := UserRoles.Get(ctx)
	if !ok {
		return NewRolesHelper(nil)
	}
	return NewRolesHelper(roles)
}

// ContextBuilder builds a context with fluent API.
type ContextBuilder struct {
	ctx context.Context
}

// NewContextBuilder creates a new context builder.
func NewContextBuilder(ctx context.Context) *ContextBuilder {
	if ctx == nil {
		ctx = context.Background()
	}
	return &ContextBuilder{ctx: ctx}
}

// With adds a typed value to the context.
func With[T any](b *ContextBuilder, key ContextKey[T], value T) *ContextBuilder {
	b.ctx = key.Set(b.ctx, value)
	return b
}

// WithUserID adds a user ID to the context.
func (b *ContextBuilder) WithUserID(userID string) *ContextBuilder {
	b.ctx = CurrentUserID.Set(b.ctx, userID)
	return b
}

// WithRoles adds roles to the context.
func (b *ContextBuilder) WithRoles(roles []string) *ContextBuilder {
	b.ctx = UserRoles.Set(b.ctx, roles)
	return b
}

// WithRequestID adds a request ID to the context.
func (b *ContextBuilder) WithRequestID(requestID string) *ContextBuilder {
	b.ctx = RequestID.Set(b.ctx, requestID)
	return b
}

// Build returns the built context.
func (b *ContextBuilder) Build() context.Context {
	return b.ctx
}

// TypedContext wraps a context with additional typed storage.
type TypedContext struct {
	context.Context
	mu   sync.RWMutex
	data map[any]any
}

// NewTypedContext creates a new typed context.
func NewTypedContext(ctx context.Context) *TypedContext {
	if ctx == nil {
		ctx = context.Background()
	}
	return &TypedContext{
		Context: ctx,
		data:    make(map[any]any),
	}
}

// Set stores a typed value.
func (c *TypedContext) Set(key, value any) {
	c.mu.Lock()
	defer c.mu.Unlock()
	c.data[key] = value
}

// Get retrieves a typed value.
func (c *TypedContext) Get(key any) (any, bool) {
	c.mu.RLock()
	defer c.mu.RUnlock()
	value, ok := c.data[key]
	return value, ok
}

// GetTyped retrieves a typed value with type assertion.
func GetTyped[T any](c *TypedContext, key any) (T, bool) {
	c.mu.RLock()
	defer c.mu.RUnlock()
	value, ok := c.data[key]
	if !ok {
		var zero T
		return zero, false
	}
	typed, ok := value.(T)
	return typed, ok
}

// SetTyped stores a typed value with type safety.
func SetTyped[T any](c *TypedContext, key ContextKey[T], value T) {
	c.mu.Lock()
	defer c.mu.Unlock()
	c.data[key] = value
}
