# API Overview

This section documents the public APIs for Better GraphQL.

## Packages

| Package | Description | Docs |
|---------|-------------|------|
| `@bgql/server` | Server-side SDK | [Server API](/api/server) |
| `@bgql/client` | Client-side SDK | [Client API](/api/client) |
| `@bgql/cli` | Command-line tools | [CLI](/cli/overview) |

## Server API

### Core Functions

```typescript
import {
  serve,
  devServer,
  createTestClient,
  defineResolvers,
  createResolver,
} from '@bgql/server'
```

- [`serve(options)`](/api/server#serve) - Start a production server
- [`devServer(options)`](/api/server#devserver) - Start a development server with hot reload
- [`createTestClient(options)`](/api/server#createtestclient) - Create a client for testing
- [`defineResolvers(resolvers)`](/api/server#defineresolvers) - Define type-safe resolvers
- [`createResolver(fn)`](/api/server#createresolver) - Create a single resolver

### Middleware

```typescript
import {
  withAuth,
  withValidation,
  withLogging,
  withCaching,
} from '@bgql/server'
```

### Error Classes

```typescript
import {
  BgqlError,
  AuthenticationError,
  ForbiddenError,
  NotFoundError,
  ValidationError,
} from '@bgql/server'
```

## Client API

### Core Functions

```typescript
import {
  createClient,
  gql,
} from '@bgql/client'
```

- [`createClient(options)`](/api/client#createclient) - Create a GraphQL client
- [`gql`](/api/client#gql) - Template tag for GraphQL documents

### Types

```typescript
import type {
  TypedDocumentNode,
  ResultOf,
  VariablesOf,
  Client,
  ClientConfig,
} from '@bgql/client'
```

### Vue Integration

```typescript
import {
  BgqlPlugin,
  BgqlProvider,
  useQuery,
  useMutation,
  useSubscription,
  useLazyQuery,
} from '@bgql/client/vue'
```

## Type Utilities

The client exports utility types for working with GraphQL results:

```typescript
import type {
  // Document types
  TypedDocumentNode,
  ResultOf,
  VariablesOf,

  // Union discrimination
  Discriminated,
  ExtractUnionMember,

  // Utility types
  DeepReadonly,
  DeepPartial,
  Option,
  Connection,
  Edge,
  PageInfo,

  // Result pattern
  Result,
  Success,
  Failure,
} from '@bgql/client'
```

## Next Steps

- [Server API Reference](/api/server)
- [Client API Reference](/api/client)
