# Client API Reference

Complete API reference for `@bgql/client`.

## Installation

```bash
npm install @bgql/client
```

## Core Functions

### createClient

Creates a GraphQL client.

```typescript
import { createClient } from '@bgql/client';

// Simple usage
const client = createClient('http://localhost:4000/graphql');

// With options
const client = createClient({
  url: 'http://localhost:4000/graphql',
  headers: {
    'Authorization': `Bearer ${token}`,
  },
  timeout: 30000,
});
```

#### Options

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `url` | `string` | - | GraphQL endpoint URL |
| `headers` | `Record<string, string>` | `{}` | Default headers |
| `timeout` | `number` | `30000` | Request timeout (ms) |
| `fetch` | `typeof fetch` | `globalThis.fetch` | Fetch implementation |
| `retry` | `RetryConfig` | - | Retry configuration |
| `credentials` | `RequestCredentials` | `'same-origin'` | Fetch credentials mode |
| `onError` | `(error: Error) => void` | - | Error callback |

#### Methods

```typescript
interface BgqlClient {
  execute<TData, TVariables>(
    operation: Operation<TVariables, TData>,
    options?: RequestOptions
  ): Promise<Result<TData, ClientError>>;

  executeTyped<TData, TVariables>(
    document: TypedDocumentNode<TData, TVariables>,
    variables: TVariables,
    options?: RequestOptions
  ): Promise<Result<TData, ClientError>>;

  query<TData, TVariables>(
    document: string,
    variables?: TVariables,
    options?: RequestOptions
  ): Promise<Result<TData, ClientError>>;

  queryTyped<TData, TVariables>(
    document: TypedDocumentNode<TData, TVariables>,
    variables: TVariables,
    options?: RequestOptions
  ): Promise<Result<TData, ClientError>>;

  mutate<TData, TVariables>(
    document: string,
    variables?: TVariables,
    options?: RequestOptions
  ): Promise<Result<TData, ClientError>>;

  mutateTyped<TData, TVariables>(
    document: TypedDocumentNode<TData, TVariables>,
    variables: TVariables,
    options?: RequestOptions
  ): Promise<Result<TData, ClientError>>;

  use(middleware: Middleware): BgqlClient;
  setHeaders(headers: Record<string, string>): void;
  setAuthToken(token: string | null): void;
}
```

### gql

Tagged template for GraphQL documents.

```typescript
import { gql } from '@bgql/client';

const GetUser = gql`
  query GetUser($id: ID!) {
    user(id: $id) {
      id
      name
    }
  }
`;

// With type parameters
const GetUser = gql<{ user: User }, { id: string }>`
  query GetUser($id: ID!) {
    user(id: $id) {
      id
      name
    }
  }
`;
```

## Types

### TypedDocumentNode

Type-safe document node.

```typescript
interface TypedDocumentNode<TData, TVariables> {
  readonly __resultType?: TData;
  readonly __variablesType?: TVariables;
  readonly __meta?: {
    readonly operationName: string;
    readonly operationType: 'query' | 'mutation' | 'subscription';
    readonly source: string;
  };
}
```

### Result Types

```typescript
type Result<T, E = Error> = Success<T> | Failure<E>;

interface Success<T> {
  readonly ok: true;
  readonly value: T;
  readonly error?: undefined;
}

interface Failure<E> {
  readonly ok: false;
  readonly value?: undefined;
  readonly error: E;
}
```

### Type Utilities

```typescript
// Extract data type from document
type ResultOf<T> = T extends TypedDocumentNode<infer TData, unknown>
  ? TData
  : never;

// Extract variables type from document
type VariablesOf<T> = T extends TypedDocumentNode<unknown, infer TVariables>
  ? TVariables
  : never;

// Option type (nullable)
type Option<T> = T | null;

// Brand type for nominal typing
type Brand<T, B extends string> = T & { readonly __brand: B };
```

## Middleware

### loggingMiddleware

Logs operations.

```typescript
import { createClient, loggingMiddleware } from '@bgql/client';

const client = createClient('http://localhost:4000/graphql')
  .use(loggingMiddleware(console.log));
```

### retryMiddleware

Retries failed requests.

```typescript
import { createClient, retryMiddleware } from '@bgql/client';

const client = createClient('http://localhost:4000/graphql')
  .use(retryMiddleware({
    maxRetries: 3,
    initialDelayMs: 1000,
    exponentialBackoff: true,
  }));
```

### cachingMiddleware

Simple response caching.

```typescript
import { createClient, cachingMiddleware } from '@bgql/client';

const cache = new Map();
const client = createClient('http://localhost:4000/graphql')
  .use(cachingMiddleware(cache, 60000)); // 60s TTL
```

### deduplicationMiddleware

Deduplicates in-flight requests.

```typescript
import { createClient, deduplicationMiddleware } from '@bgql/client';

const client = createClient('http://localhost:4000/graphql')
  .use(deduplicationMiddleware());
```

### batchingMiddleware

Batches multiple queries.

```typescript
import { createClient, batchingMiddleware } from '@bgql/client';

const client = createClient('http://localhost:4000/graphql')
  .use(batchingMiddleware({
    maxBatchSize: 10,
    batchInterval: 10,
  }));
```

### normalizedCacheMiddleware

Normalized entity caching.

```typescript
import {
  createClient,
  createNormalizedCache,
  normalizedCacheMiddleware,
} from '@bgql/client';

const cache = createNormalizedCache({ ttlMs: 300000 });
const client = createClient('http://localhost:4000/graphql')
  .use(normalizedCacheMiddleware(cache));
```

## Union Helpers

### matchUnion

Pattern match on discriminated unions.

```typescript
import { matchUnion } from '@bgql/client';

const message = matchUnion(result.user, {
  User: (user) => `Hello, ${user.name}!`,
  NotFoundError: (err) => `Not found: ${err.message}`,
  AuthError: (err) => `Auth failed: ${err.message}`,
});
```

### matchUnionPartial

Partial match with fallback.

```typescript
import { matchUnionPartial } from '@bgql/client';

const message = matchUnionPartial(
  result.user,
  {
    User: (user) => `Hello, ${user.name}!`,
  },
  (other) => `Error: ${other.message}`
);
```

### isTypename

Type guard for discriminated unions.

```typescript
import { isTypename } from '@bgql/client';

if (isTypename('User')(result.user)) {
  console.log(result.user.name); // Typed as User
}
```

### assertNever

Exhaustiveness check.

```typescript
import { assertNever } from '@bgql/client';

switch (result.__typename) {
  case 'User': return handleUser(result);
  case 'Error': return handleError(result);
  default: return assertNever(result);
}
```

## Result Helpers

### ok / err

Create Result values.

```typescript
import { ok, err } from '@bgql/client';

const success = ok({ user: { name: 'John' } });
const failure = err(new Error('Not found'));
```

### isOk / isErr

Type guards for Result.

```typescript
import { isOk, isErr } from '@bgql/client';

const result = await client.query(GetUser, { id: '1' });

if (isOk(result)) {
  console.log(result.value.user);
}

if (isErr(result)) {
  console.error(result.error);
}
```

### mapResult / mapError

Transform Result values.

```typescript
import { mapResult, mapError } from '@bgql/client';

const mapped = mapResult(result, (data) => data.user.name);
const withNewError = mapError(result, (err) => new CustomError(err));
```

### unwrap / unwrapOr

Extract values from Result.

```typescript
import { unwrap, unwrapOr } from '@bgql/client';

// Throws on error
const value = unwrap(result);

// Returns default on error
const value = unwrapOr(result, defaultUser);
```

## Option Helpers

### isSome / isNone

Check Option values.

```typescript
import { isSome, isNone } from '@bgql/client';

if (isSome(user.bio)) {
  console.log(user.bio);
}
```

### mapOption

Transform Option values.

```typescript
import { mapOption } from '@bgql/client';

const upperBio = mapOption(user.bio, (bio) => bio.toUpperCase());
```

### unwrapOption

Extract with default.

```typescript
import { unwrapOption } from '@bgql/client';

const bio = unwrapOption(user.bio, 'No bio provided');
```

## Connection Helpers

### extractNodes

Get nodes from connection.

```typescript
import { extractNodes } from '@bgql/client';

const users = extractNodes(data.users);
// users: User[] (extracted from edges)
```

## Vue Integration

See [Vue.js Integration](/frontend/vue) for the complete Vue API.

```typescript
import {
  BgqlPlugin,
  BgqlProvider,
  useQuery,
  useMutation,
  useSubscription,
  useLazyQuery,
} from '@bgql/client/vue';
```

## Error Types

### ClientError

Base client error.

```typescript
interface ClientError {
  readonly type: 'network' | 'graphql' | 'abort' | 'timeout' | 'unknown';
  readonly message: string;
  readonly code?: string;
  readonly retryable: boolean;
  readonly cause?: Error;
}
```

### Error Constructors

```typescript
import {
  networkError,
  graphqlExecutionError,
  abortError,
  timeoutError,
  unknownError,
} from '@bgql/client';
```

## Configuration Types

```typescript
interface ClientConfig {
  readonly url: string;
  readonly headers?: Record<string, string>;
  readonly timeout?: number;
  readonly fetch?: typeof fetch;
  readonly retry?: RetryConfig;
  readonly credentials?: RequestCredentials;
  readonly onError?: (error: unknown) => void;
}

interface RetryConfig {
  readonly maxRetries?: number;
  readonly initialDelayMs?: number;
  readonly maxDelayMs?: number;
  readonly exponentialBackoff?: boolean;
  readonly shouldRetry?: (error: unknown, attempt: number) => boolean;
}

interface RequestOptions {
  readonly signal?: AbortSignal;
  readonly headers?: Record<string, string>;
  readonly timeout?: number;
  readonly context?: Record<string, unknown>;
}
```

## Next Steps

- [Server API](/api/server)
- [Vue.js Integration](/frontend/vue)
- [Quick Start](/frontend/quickstart)
