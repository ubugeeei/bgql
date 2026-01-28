// Package server provides a type-safe GraphQL server.
package server

import (
	"context"
	"encoding/json"
	"fmt"
	"net/http"
	"sync"
	"time"

	"github.com/ubugeeei/bgql/bindings/go/bgql/result"
)

// Config holds server configuration.
type Config struct {
	Port           int
	Host           string
	Introspection  bool
	Playground     bool
	PlaygroundPath string
	MaxDepth       int
	MaxComplexity  int
	Timeout        time.Duration
}

// DefaultConfig returns default server configuration.
func DefaultConfig() Config {
	return Config{
		Port:           4000,
		Host:           "localhost",
		Introspection:  true,
		Playground:     true,
		PlaygroundPath: "/playground",
		MaxDepth:       10,
		MaxComplexity:  1000,
		Timeout:        30 * time.Second,
	}
}

// Request represents an incoming GraphQL request.
type Request struct {
	Query         string         `json:"query"`
	Variables     map[string]any `json:"variables,omitempty"`
	OperationName string         `json:"operationName,omitempty"`
}

// Response represents a GraphQL response.
type Response struct {
	Data   any            `json:"data,omitempty"`
	Errors []GraphQLError `json:"errors,omitempty"`
}

// GraphQLError represents a GraphQL error.
type GraphQLError struct {
	Message    string         `json:"message"`
	Path       []any          `json:"path,omitempty"`
	Locations  []Location     `json:"locations,omitempty"`
	Extensions map[string]any `json:"extensions,omitempty"`
}

// Location represents a location in a GraphQL document.
type Location struct {
	Line   int `json:"line"`
	Column int `json:"column"`
}

// Context holds request-scoped data.
type Context struct {
	context.Context
	Request *http.Request
	Loaders *LoaderStore
	Data    map[string]any
}

// NewContext creates a new context.
func NewContext(ctx context.Context, req *http.Request) *Context {
	return &Context{
		Context: ctx,
		Request: req,
		Loaders: NewLoaderStore(),
		Data:    make(map[string]any),
	}
}

// Set stores a value in the context.
func (c *Context) Set(key string, value any) {
	c.Data[key] = value
}

// Get retrieves a value from the context.
func (c *Context) Get(key string) (any, bool) {
	v, ok := c.Data[key]
	return v, ok
}

// GetString retrieves a string value from the context.
func (c *Context) GetString(key string) string {
	if v, ok := c.Data[key]; ok {
		if s, ok := v.(string); ok {
			return s
		}
	}
	return ""
}

// Server is the GraphQL server.
type Server struct {
	config      Config
	schema      string
	resolvers   map[string]map[string]ResolverFn
	middlewares []Middleware
	httpServer  *http.Server
}

// ResolverFn is a resolver function type.
type ResolverFn func(ctx *Context, parent any, args map[string]any) (any, error)

// Middleware is a server middleware function.
type Middleware func(ctx *Context, next func(*Context) *Response) *Response

// Builder is a server builder.
type Builder struct {
	config    Config
	schema    string
	resolvers map[string]map[string]ResolverFn
}

// NewBuilder creates a new server builder.
func NewBuilder() *Builder {
	return &Builder{
		config:    DefaultConfig(),
		resolvers: make(map[string]map[string]ResolverFn),
	}
}

// Config sets the server configuration.
func (b *Builder) Config(config Config) *Builder {
	b.config = config
	return b
}

// Port sets the server port.
func (b *Builder) Port(port int) *Builder {
	b.config.Port = port
	return b
}

// Schema sets the schema from SDL.
func (b *Builder) Schema(sdl string) *Builder {
	b.schema = sdl
	return b
}

// Resolver adds a resolver.
func (b *Builder) Resolver(typeName, fieldName string, fn ResolverFn) *Builder {
	if b.resolvers[typeName] == nil {
		b.resolvers[typeName] = make(map[string]ResolverFn)
	}
	b.resolvers[typeName][fieldName] = fn
	return b
}

// EnablePlayground enables the GraphQL playground.
func (b *Builder) EnablePlayground(path string) *Builder {
	b.config.Playground = true
	if path != "" {
		b.config.PlaygroundPath = path
	}
	return b
}

// DisablePlayground disables the GraphQL playground.
func (b *Builder) DisablePlayground() *Builder {
	b.config.Playground = false
	return b
}

// Build creates the server.
func (b *Builder) Build() result.Result[*Server] {
	if b.schema == "" {
		return result.ErrMsg[*Server]("schema is required")
	}

	return result.Ok(&Server{
		config:    b.config,
		schema:    b.schema,
		resolvers: b.resolvers,
	})
}

// Use adds middleware to the server.
func (s *Server) Use(middleware Middleware) *Server {
	s.middlewares = append(s.middlewares, middleware)
	return s
}

// Listen starts the server.
func (s *Server) Listen() error {
	mux := http.NewServeMux()

	// GraphQL endpoint
	mux.HandleFunc("/graphql", s.handleGraphQL)

	// Playground endpoint (if enabled)
	if s.config.Playground {
		mux.HandleFunc(s.config.PlaygroundPath, s.handlePlayground)
	}

	addr := fmt.Sprintf("%s:%d", s.config.Host, s.config.Port)

	s.httpServer = &http.Server{
		Addr:         addr,
		Handler:      mux,
		ReadTimeout:  s.config.Timeout,
		WriteTimeout: s.config.Timeout,
	}

	fmt.Printf("[bgql] Server starting on http://%s\n", addr)
	if s.config.Playground {
		fmt.Printf("[bgql] Playground available at http://%s%s\n", addr, s.config.PlaygroundPath)
	}

	return s.httpServer.ListenAndServe()
}

// Stop stops the server.
func (s *Server) Stop(ctx context.Context) error {
	if s.httpServer != nil {
		return s.httpServer.Shutdown(ctx)
	}
	return nil
}

func (s *Server) handleGraphQL(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodPost {
		http.Error(w, "Method not allowed", http.StatusMethodNotAllowed)
		return
	}

	var req Request
	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		http.Error(w, "Invalid request body", http.StatusBadRequest)
		return
	}

	ctx := NewContext(r.Context(), r)

	// Execute query
	resp := s.execute(ctx, &req)

	// Write response
	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(resp)
}

func (s *Server) handlePlayground(w http.ResponseWriter, r *http.Request) {
	w.Header().Set("Content-Type", "text/html")
	w.Write([]byte(playgroundHTML))
}

func (s *Server) execute(ctx *Context, req *Request) *Response {
	// Build middleware chain
	handler := func(ctx *Context) *Response {
		return s.doExecute(ctx, req)
	}

	for i := len(s.middlewares) - 1; i >= 0; i-- {
		middleware := s.middlewares[i]
		next := handler
		handler = func(ctx *Context) *Response {
			return middleware(ctx, next)
		}
	}

	return handler(ctx)
}

func (s *Server) doExecute(ctx *Context, req *Request) *Response {
	// TODO: Implement actual GraphQL execution
	// For now, return a placeholder response
	return &Response{
		Errors: []GraphQLError{
			{Message: "Execution not yet implemented"},
		},
	}
}

// Playground HTML template
const playgroundHTML = `<!DOCTYPE html>
<html>
<head>
  <meta charset="utf-8">
  <title>bgql Playground</title>
  <link rel="stylesheet" href="https://cdn.jsdelivr.net/npm/graphiql@3/graphiql.min.css" />
  <style>
    body { margin: 0; height: 100vh; }
    #graphiql { height: 100vh; }
  </style>
</head>
<body>
  <div id="graphiql">Loading...</div>
  <script crossorigin src="https://cdn.jsdelivr.net/npm/react@18/umd/react.production.min.js"></script>
  <script crossorigin src="https://cdn.jsdelivr.net/npm/react-dom@18/umd/react-dom.production.min.js"></script>
  <script crossorigin src="https://cdn.jsdelivr.net/npm/graphiql@3/graphiql.min.js"></script>
  <script>
    const root = ReactDOM.createRoot(document.getElementById('graphiql'));
    root.render(
      React.createElement(GraphiQL, {
        fetcher: GraphiQL.createFetcher({ url: '/graphql' }),
        defaultEditorToolsVisibility: true,
      })
    );
  </script>
</body>
</html>`

// =============================================================================
// DataLoader
// =============================================================================

// DataLoader batches and caches data loading.
type DataLoader[K comparable, V any] struct {
	batchFn     func(keys []K) (map[K]V, error)
	cache       map[K]V
	batch       []K
	batchChan   chan struct{}
	mu          sync.Mutex
	maxBatchSize int
}

// NewDataLoader creates a new DataLoader.
func NewDataLoader[K comparable, V any](batchFn func(keys []K) (map[K]V, error)) *DataLoader[K, V] {
	return &DataLoader[K, V]{
		batchFn:     batchFn,
		cache:       make(map[K]V),
		maxBatchSize: 100,
	}
}

// Load loads a single value by key.
func (dl *DataLoader[K, V]) Load(ctx context.Context, key K) (V, error) {
	dl.mu.Lock()

	// Check cache
	if v, ok := dl.cache[key]; ok {
		dl.mu.Unlock()
		return v, nil
	}

	dl.mu.Unlock()

	// For simplicity, just call batch function directly
	// In production, this would batch requests across the same tick
	result, err := dl.batchFn([]K{key})
	if err != nil {
		var zero V
		return zero, err
	}

	dl.mu.Lock()
	defer dl.mu.Unlock()

	if v, ok := result[key]; ok {
		dl.cache[key] = v
		return v, nil
	}

	var zero V
	return zero, fmt.Errorf("key not found: %v", key)
}

// LoadMany loads multiple values by keys.
func (dl *DataLoader[K, V]) LoadMany(ctx context.Context, keys []K) ([]V, []error) {
	values := make([]V, len(keys))
	errors := make([]error, len(keys))

	for i, key := range keys {
		v, err := dl.Load(ctx, key)
		values[i] = v
		errors[i] = err
	}

	return values, errors
}

// Clear clears a key from the cache.
func (dl *DataLoader[K, V]) Clear(key K) {
	dl.mu.Lock()
	defer dl.mu.Unlock()
	delete(dl.cache, key)
}

// ClearAll clears all keys from the cache.
func (dl *DataLoader[K, V]) ClearAll() {
	dl.mu.Lock()
	defer dl.mu.Unlock()
	dl.cache = make(map[K]V)
}

// Prime primes the cache with a value.
func (dl *DataLoader[K, V]) Prime(key K, value V) {
	dl.mu.Lock()
	defer dl.mu.Unlock()
	dl.cache[key] = value
}

// LoaderStore stores DataLoaders per request.
type LoaderStore struct {
	loaders map[string]any
	mu      sync.RWMutex
}

// NewLoaderStore creates a new loader store.
func NewLoaderStore() *LoaderStore {
	return &LoaderStore{
		loaders: make(map[string]any),
	}
}

// Get gets or creates a DataLoader.
func GetLoader[K comparable, V any](store *LoaderStore, name string, batchFn func(keys []K) (map[K]V, error)) *DataLoader[K, V] {
	store.mu.RLock()
	if loader, ok := store.loaders[name]; ok {
		store.mu.RUnlock()
		return loader.(*DataLoader[K, V])
	}
	store.mu.RUnlock()

	store.mu.Lock()
	defer store.mu.Unlock()

	// Double check
	if loader, ok := store.loaders[name]; ok {
		return loader.(*DataLoader[K, V])
	}

	loader := NewDataLoader(batchFn)
	store.loaders[name] = loader
	return loader
}

// ClearAll clears all loaders.
func (s *LoaderStore) ClearAll() {
	s.mu.Lock()
	defer s.mu.Unlock()
	s.loaders = make(map[string]any)
}

// =============================================================================
// Built-in Middleware
// =============================================================================

// LoggingMiddleware logs requests.
func LoggingMiddleware(logger func(format string, args ...any)) Middleware {
	if logger == nil {
		logger = func(format string, args ...any) {
			fmt.Printf(format+"\n", args...)
		}
	}

	return func(ctx *Context, next func(*Context) *Response) *Response {
		start := time.Now()
		logger("[bgql] Request started: %s", ctx.Request.URL.Path)

		resp := next(ctx)

		duration := time.Since(start)
		hasErrors := len(resp.Errors) > 0
		logger("[bgql] Request completed in %v (hasErrors: %v)", duration, hasErrors)

		return resp
	}
}

// RateLimitMiddleware limits request rate.
func RateLimitMiddleware(windowMs time.Duration, maxRequests int) Middleware {
	var mu sync.Mutex
	requests := make(map[string]struct {
		count     int
		resetTime time.Time
	})

	return func(ctx *Context, next func(*Context) *Response) *Response {
		ip := ctx.Request.RemoteAddr

		mu.Lock()
		now := time.Now()
		entry, ok := requests[ip]
		if !ok || now.After(entry.resetTime) {
			entry = struct {
				count     int
				resetTime time.Time
			}{
				count:     0,
				resetTime: now.Add(windowMs),
			}
		}
		entry.count++
		requests[ip] = entry
		mu.Unlock()

		if entry.count > maxRequests {
			return &Response{
				Errors: []GraphQLError{
					{
						Message: "Rate limit exceeded",
						Extensions: map[string]any{
							"code":       "RATE_LIMITED",
							"retryAfter": entry.resetTime.Sub(now).Milliseconds(),
						},
					},
				},
			}
		}

		return next(ctx)
	}
}
