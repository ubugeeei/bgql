// Package bgql provides a type-safe GraphQL SDK for Go.
//
// This package provides:
//   - Type-safe GraphQL client with middleware support
//   - Type-safe GraphQL server with DataLoader integration
//   - Result type for error handling (no panics)
//   - Built-in playground support
//
// # Client Example
//
//	client := client.New("http://localhost:4000/graphql")
//	client.SetAuthToken("your-token")
//
//	resp := client.Query(ctx, `
//	    query GetUser($id: ID!) {
//	        user(id: $id) {
//	            id
//	            name
//	        }
//	    }
//	`, map[string]any{"id": "1"})
//
//	if resp.IsOk() {
//	    fmt.Println(resp.Unwrap().Data)
//	} else {
//	    fmt.Println("Error:", resp.Error())
//	}
//
// # Server Example
//
//	srv := server.NewBuilder().
//	    Port(4000).
//	    Schema(`
//	        type Query {
//	            hello: String
//	        }
//	    `).
//	    Resolver("Query", "hello", func(ctx *server.Context, parent any, args map[string]any) (any, error) {
//	        return "Hello, World!", nil
//	    }).
//	    EnablePlayground("/playground").
//	    Build()
//
//	if srv.IsErr() {
//	    log.Fatal(srv.Error())
//	}
//
//	srv.Unwrap().Listen()
//
// # Result Type
//
//	result := result.Ok("success")
//	result := result.Err[string](errors.New("failed"))
//
//	// Pattern matching
//	value := result.Match(result,
//	    func(v string) string { return "Got: " + v },
//	    func(e error) string { return "Error: " + e.Error() },
//	)
package bgql

import (
	"github.com/ubugeeei/bgql/bindings/go/bgql/client"
	"github.com/ubugeeei/bgql/bindings/go/bgql/result"
	"github.com/ubugeeei/bgql/bindings/go/bgql/server"
)

// Version returns the bgql version.
const Version = "0.1.0"

// Re-export client types
type (
	Client       = client.Client
	ClientConfig = client.Config
)

// Re-export server types
type (
	Server       = server.Server
	ServerConfig = server.Config
	Context      = server.Context
	ResolverFn   = server.ResolverFn
)

// Re-export result types
type Result[T any] = result.Result[T]

// NewClient creates a new GraphQL client.
func NewClient(url string) *Client {
	return client.New(url)
}

// NewClientWithConfig creates a new GraphQL client with custom configuration.
func NewClientWithConfig(config ClientConfig) *Client {
	return client.NewWithConfig(config)
}

// NewServerBuilder creates a new server builder.
func NewServerBuilder() *server.Builder {
	return server.NewBuilder()
}

// Ok creates a successful Result.
func Ok[T any](value T) Result[T] {
	return result.Ok(value)
}

// Err creates a failed Result.
func Err[T any](err error) Result[T] {
	return result.Err[T](err)
}
