// Package client provides a type-safe GraphQL client.
package client

import (
	"bytes"
	"context"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"time"

	"github.com/ubugeeei/bgql/bindings/go/bgql/result"
)

// Config holds client configuration.
type Config struct {
	URL           string
	Timeout       time.Duration
	Headers       map[string]string
	MaxRetries    int
	RetryInterval time.Duration
	HTTPClient    *http.Client
}

// DefaultConfig returns default client configuration.
func DefaultConfig(url string) Config {
	return Config{
		URL:           url,
		Timeout:       30 * time.Second,
		Headers:       make(map[string]string),
		MaxRetries:    3,
		RetryInterval: time.Second,
	}
}

// Request represents a GraphQL request.
type Request struct {
	Query         string         `json:"query"`
	Variables     map[string]any `json:"variables,omitempty"`
	OperationName string         `json:"operationName,omitempty"`
}

// Response represents a GraphQL response.
type Response struct {
	Data   json.RawMessage `json:"data,omitempty"`
	Errors []GraphQLError  `json:"errors,omitempty"`
}

// GraphQLError represents a GraphQL error.
type GraphQLError struct {
	Message    string         `json:"message"`
	Path       []any          `json:"path,omitempty"`
	Locations  []Location     `json:"locations,omitempty"`
	Extensions map[string]any `json:"extensions,omitempty"`
}

func (e GraphQLError) Error() string {
	return e.Message
}

// Location represents a location in a GraphQL document.
type Location struct {
	Line   int `json:"line"`
	Column int `json:"column"`
}

// Client is the GraphQL client.
type Client struct {
	config      Config
	httpClient  *http.Client
	middlewares []Middleware
}

// Middleware is a function that wraps request execution.
type Middleware func(ctx context.Context, req *Request, next func(context.Context, *Request) (*Response, error)) (*Response, error)

// New creates a new GraphQL client.
func New(url string) *Client {
	return NewWithConfig(DefaultConfig(url))
}

// NewWithConfig creates a new GraphQL client with custom configuration.
func NewWithConfig(config Config) *Client {
	httpClient := config.HTTPClient
	if httpClient == nil {
		httpClient = &http.Client{
			Timeout: config.Timeout,
		}
	}

	return &Client{
		config:     config,
		httpClient: httpClient,
	}
}

// Use adds middleware to the client.
func (c *Client) Use(middleware Middleware) *Client {
	c.middlewares = append(c.middlewares, middleware)
	return c
}

// SetHeader sets a default header.
func (c *Client) SetHeader(key, value string) *Client {
	c.config.Headers[key] = value
	return c
}

// SetAuthToken sets the Authorization header with a Bearer token.
func (c *Client) SetAuthToken(token string) *Client {
	if token != "" {
		c.config.Headers["Authorization"] = "Bearer " + token
	} else {
		delete(c.config.Headers, "Authorization")
	}
	return c
}

// Query executes a GraphQL query.
func (c *Client) Query(ctx context.Context, query string, variables map[string]any) result.Result[*Response] {
	return c.Execute(ctx, &Request{
		Query:     query,
		Variables: variables,
	})
}

// Mutate executes a GraphQL mutation.
func (c *Client) Mutate(ctx context.Context, mutation string, variables map[string]any) result.Result[*Response] {
	return c.Execute(ctx, &Request{
		Query:     mutation,
		Variables: variables,
	})
}

// Execute executes a GraphQL request.
func (c *Client) Execute(ctx context.Context, req *Request) result.Result[*Response] {
	// Build middleware chain
	handler := c.doRequest

	for i := len(c.middlewares) - 1; i >= 0; i-- {
		middleware := c.middlewares[i]
		next := handler
		handler = func(ctx context.Context, req *Request) (*Response, error) {
			return middleware(ctx, req, next)
		}
	}

	resp, err := handler(ctx, req)
	if err != nil {
		return result.Err[*Response](err)
	}

	// Check for GraphQL errors
	if len(resp.Errors) > 0 {
		return result.Err[*Response](&resp.Errors[0])
	}

	return result.Ok(resp)
}

// ExecuteInto executes a request and unmarshals the data into the target.
func ExecuteInto[T any](c *Client, ctx context.Context, req *Request) result.Result[T] {
	resp := c.Execute(ctx, req)
	if resp.IsErr() {
		return result.Err[T](resp.Error())
	}

	var data T
	if err := json.Unmarshal(resp.Unwrap().Data, &data); err != nil {
		return result.Err[T](fmt.Errorf("failed to unmarshal response: %w", err))
	}

	return result.Ok(data)
}

func (c *Client) doRequest(ctx context.Context, req *Request) (*Response, error) {
	body, err := json.Marshal(req)
	if err != nil {
		return nil, fmt.Errorf("failed to marshal request: %w", err)
	}

	httpReq, err := http.NewRequestWithContext(ctx, "POST", c.config.URL, bytes.NewReader(body))
	if err != nil {
		return nil, fmt.Errorf("failed to create request: %w", err)
	}

	httpReq.Header.Set("Content-Type", "application/json")
	httpReq.Header.Set("Accept", "application/json")

	for k, v := range c.config.Headers {
		httpReq.Header.Set(k, v)
	}

	httpResp, err := c.httpClient.Do(httpReq)
	if err != nil {
		return nil, fmt.Errorf("request failed: %w", err)
	}
	defer httpResp.Body.Close()

	respBody, err := io.ReadAll(httpResp.Body)
	if err != nil {
		return nil, fmt.Errorf("failed to read response: %w", err)
	}

	if httpResp.StatusCode >= 400 {
		return nil, fmt.Errorf("HTTP %d: %s", httpResp.StatusCode, string(respBody))
	}

	var resp Response
	if err := json.Unmarshal(respBody, &resp); err != nil {
		return nil, fmt.Errorf("failed to unmarshal response: %w", err)
	}

	return &resp, nil
}

// =============================================================================
// Middleware Helpers
// =============================================================================

// LoggingMiddleware logs requests and responses.
func LoggingMiddleware(logger func(format string, args ...any)) Middleware {
	if logger == nil {
		logger = func(format string, args ...any) {
			fmt.Printf(format+"\n", args...)
		}
	}

	return func(ctx context.Context, req *Request, next func(context.Context, *Request) (*Response, error)) (*Response, error) {
		start := time.Now()
		logger("[bgql] query: %s", req.OperationName)

		resp, err := next(ctx, req)

		duration := time.Since(start)
		if err != nil {
			logger("[bgql] %s failed after %v: %v", req.OperationName, duration, err)
		} else if len(resp.Errors) > 0 {
			logger("[bgql] %s completed with errors in %v", req.OperationName, duration)
		} else {
			logger("[bgql] %s completed in %v", req.OperationName, duration)
		}

		return resp, err
	}
}

// RetryMiddleware retries failed requests.
func RetryMiddleware(maxRetries int, interval time.Duration) Middleware {
	return func(ctx context.Context, req *Request, next func(context.Context, *Request) (*Response, error)) (*Response, error) {
		var lastErr error

		for attempt := 0; attempt <= maxRetries; attempt++ {
			resp, err := next(ctx, req)
			if err == nil {
				return resp, nil
			}

			lastErr = err

			if attempt < maxRetries {
				select {
				case <-ctx.Done():
					return nil, ctx.Err()
				case <-time.After(interval):
				}
			}
		}

		return nil, lastErr
	}
}

// CachingMiddleware caches query responses.
func CachingMiddleware(cache Cache, ttl time.Duration) Middleware {
	return func(ctx context.Context, req *Request, next func(context.Context, *Request) (*Response, error)) (*Response, error) {
		// Generate cache key
		key := fmt.Sprintf("%s:%v", req.Query, req.Variables)

		// Check cache
		if cached, ok := cache.Get(key); ok {
			return cached, nil
		}

		// Execute request
		resp, err := next(ctx, req)
		if err != nil {
			return nil, err
		}

		// Cache successful responses without errors
		if len(resp.Errors) == 0 {
			cache.Set(key, resp, ttl)
		}

		return resp, nil
	}
}

// Cache interface for caching middleware.
type Cache interface {
	Get(key string) (*Response, bool)
	Set(key string, value *Response, ttl time.Duration)
}

// SimpleCache is a basic in-memory cache implementation.
type SimpleCache struct {
	data map[string]cacheEntry
}

type cacheEntry struct {
	response  *Response
	expiresAt time.Time
}

// NewSimpleCache creates a new simple cache.
func NewSimpleCache() *SimpleCache {
	return &SimpleCache{
		data: make(map[string]cacheEntry),
	}
}

func (c *SimpleCache) Get(key string) (*Response, bool) {
	entry, ok := c.data[key]
	if !ok || time.Now().After(entry.expiresAt) {
		return nil, false
	}
	return entry.response, true
}

func (c *SimpleCache) Set(key string, value *Response, ttl time.Duration) {
	c.data[key] = cacheEntry{
		response:  value,
		expiresAt: time.Now().Add(ttl),
	}
}
