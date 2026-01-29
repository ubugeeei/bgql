package sdk

import (
	"bytes"
	"context"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"time"
)

// Operation represents a typed GraphQL operation.
type Operation[TVariables, TData any] struct {
	Query         string
	OperationName string
}

// NewQuery creates a new query operation.
func NewQuery[TVariables, TData any](operationName, query string) Operation[TVariables, TData] {
	return Operation[TVariables, TData]{
		Query:         query,
		OperationName: operationName,
	}
}

// NewMutation creates a new mutation operation.
func NewMutation[TVariables, TData any](operationName, query string) Operation[TVariables, TData] {
	return Operation[TVariables, TData]{
		Query:         query,
		OperationName: operationName,
	}
}

// GraphQLRequest is the JSON structure sent to the server.
type GraphQLRequest struct {
	Query         string `json:"query"`
	Variables     any    `json:"variables,omitempty"`
	OperationName string `json:"operationName,omitempty"`
}

// GraphQLResponse is the JSON structure received from the server.
type GraphQLResponse[T any] struct {
	Data   *T             `json:"data,omitempty"`
	Errors []GraphQLError `json:"errors,omitempty"`
}

// HasErrors returns true if there are errors.
func (r *GraphQLResponse[T]) HasErrors() bool {
	return len(r.Errors) > 0
}

// ClientConfig configures the GraphQL client.
type ClientConfig struct {
	URL          string
	Timeout      time.Duration
	MaxRetries   int
	RetryDelay   time.Duration
	Headers      http.Header
	HTTPClient   *http.Client
}

// DefaultConfig returns default client configuration.
func DefaultConfig(url string) ClientConfig {
	return ClientConfig{
		URL:        url,
		Timeout:    30 * time.Second,
		MaxRetries: 3,
		RetryDelay: 100 * time.Millisecond,
		Headers:    make(http.Header),
	}
}

// Client is a strongly typed GraphQL client.
type Client struct {
	config     ClientConfig
	httpClient *http.Client
}

// NewClient creates a new GraphQL client.
func NewClient(config ClientConfig) *Client {
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

// Execute executes a typed operation.
func Execute[TVariables, TData any](
	c *Client,
	ctx context.Context,
	op Operation[TVariables, TData],
	variables TVariables,
) Result[TData] {
	response, err := ExecuteRaw[TData](c, ctx, op.Query, variables, op.OperationName)
	if err != nil {
		return Err[TData](err)
	}

	if response.HasErrors() {
		return Err[TData](NewError(ErrExecutionError, response.Errors[0].Message).
			WithExtension("graphqlErrors", response.Errors))
	}

	if response.Data == nil {
		return Err[TData](NewError(ErrNoData, "No data in response"))
	}

	return Ok(*response.Data)
}

// ExecuteRaw executes a raw GraphQL query.
func ExecuteRaw[TData any](
	c *Client,
	ctx context.Context,
	query string,
	variables any,
	operationName string,
) (*GraphQLResponse[TData], error) {
	var lastErr error = NewError(ErrNetworkError, "No attempts made")

	for attempt := 0; attempt <= c.config.MaxRetries; attempt++ {
		if attempt > 0 {
			delay := c.config.RetryDelay * time.Duration(1<<(attempt-1))
			select {
			case <-ctx.Done():
				return nil, ctx.Err()
			case <-time.After(delay):
			}
		}

		response, err := c.doRequest(ctx, query, variables, operationName)
		if err == nil {
			var parsed GraphQLResponse[TData]
			if err := json.Unmarshal(response, &parsed); err != nil {
				return nil, NewError(ErrParseError, "Failed to parse response").WithCause(err)
			}
			return &parsed, nil
		}

		lastErr = err

		// Only retry on retryable errors
		if sdkErr, ok := AsSdkError(err); ok {
			if !sdkErr.Code.IsRetryable() {
				return nil, err
			}
		}
	}

	return nil, lastErr
}

func (c *Client) doRequest(
	ctx context.Context,
	query string,
	variables any,
	operationName string,
) ([]byte, error) {
	reqBody := GraphQLRequest{
		Query:         query,
		Variables:     variables,
		OperationName: operationName,
	}

	body, err := json.Marshal(reqBody)
	if err != nil {
		return nil, NewError(ErrParseError, "Failed to marshal request").WithCause(err)
	}

	req, err := http.NewRequestWithContext(ctx, "POST", c.config.URL, bytes.NewReader(body))
	if err != nil {
		return nil, NewError(ErrNetworkError, "Failed to create request").WithCause(err)
	}

	req.Header.Set("Content-Type", "application/json")
	for key, values := range c.config.Headers {
		for _, value := range values {
			req.Header.Add(key, value)
		}
	}

	resp, err := c.httpClient.Do(req)
	if err != nil {
		if ctx.Err() != nil {
			return nil, NewError(ErrTimeout, "Request timed out").WithCause(ctx.Err())
		}
		return nil, NewError(ErrNetworkError, "Request failed").WithCause(err)
	}
	defer resp.Body.Close()

	if resp.StatusCode < 200 || resp.StatusCode >= 300 {
		return nil, NewError(ErrHttpError, fmt.Sprintf("HTTP %d", resp.StatusCode)).
			WithExtension("status", resp.StatusCode)
	}

	responseBody, err := io.ReadAll(resp.Body)
	if err != nil {
		return nil, NewError(ErrNetworkError, "Failed to read response").WithCause(err)
	}

	return responseBody, nil
}

// WithHeader adds a header to the client.
func (c *Client) WithHeader(key, value string) *Client {
	c.config.Headers.Add(key, value)
	return c
}

// WithHeaders sets all headers.
func (c *Client) WithHeaders(headers http.Header) *Client {
	c.config.Headers = headers
	return c
}
