# Queries

Learn how to fetch data with Better GraphQL's type-safe query system.

## Basic Queries

### Simple Query

```typescript
import { createClient, gql } from '@bgql/client';

const client = createClient('http://localhost:4000/graphql');

const result = await client.query(`
  query {
    users {
      id
      name
    }
  }
`);

if (result.ok) {
  console.log(result.value.users);
}
```

### Query with Variables

```typescript
const result = await client.query(
  `
    query GetUser($id: ID!) {
      user(id: $id) {
        id
        name
        email
      }
    }
  `,
  { id: '1' }
);
```

## Typed Queries

### Using TypedDocumentNode

```typescript
import { gql } from '@bgql/client';

// Define typed query
const GetUserDocument = gql<
  { user: { id: string; name: string } },
  { id: string }
>`
  query GetUser($id: ID!) {
    user(id: $id) {
      id
      name
    }
  }
`;

// Full type inference
const result = await client.queryTyped(GetUserDocument, { id: '1' });

if (result.ok) {
  console.log(result.value.user.name); // string
}
```

### Generated Types (Recommended)

```bash
# Generate types from schema
bgql codegen ./schema.bgql --lang typescript -o ./generated
```

```typescript
import { GetUserDocument, GetUsersDocument } from './generated/graphql';

// Types are inferred from generated document
const result = await client.queryTyped(GetUserDocument, { id: '1' });
```

## Handling Results

### Result Pattern

All queries return a `Result<T, E>` type:

```typescript
const result = await client.query(GetUser, { id: '1' });

// Check success
if (result.ok) {
  // result.value is the data
  console.log(result.value.user);
} else {
  // result.error is the error
  console.error(result.error.message);
}
```

### Using Helper Functions

```typescript
import { isOk, isErr, unwrapOr, match } from '@bgql/client';

// Type guards
if (isOk(result)) {
  console.log(result.value);
}

if (isErr(result)) {
  console.error(result.error);
}

// Unwrap with default
const users = unwrapOr(result, { users: [] }).users;

// Pattern matching
const message = match(result, {
  ok: (data) => `Found ${data.users.length} users`,
  err: (error) => `Error: ${error.message}`,
});
```

## Union Type Results

### Querying Unions

```graphql
# Schema
union UserResult = User | NotFoundError | AuthError

type Query {
  user(id: ID!): UserResult
}
```

```typescript
const result = await client.query(`
  query GetUser($id: ID!) {
    user(id: $id) {
      ... on User {
        id
        name
        email
      }
      ... on NotFoundError {
        message
        resourceId
      }
      ... on AuthError {
        message
      }
    }
  }
`, { id: '1' });
```

### Type-Safe Union Handling

```typescript
import { matchUnion, isTypename } from '@bgql/client';

if (result.ok) {
  // Pattern matching
  const message = matchUnion(result.value.user, {
    User: (user) => `Hello, ${user.name}!`,
    NotFoundError: (err) => `User not found: ${err.resourceId}`,
    AuthError: (err) => `Auth failed: ${err.message}`,
  });

  // Type guard
  if (isTypename('User')(result.value.user)) {
    console.log(result.value.user.name); // Typed as string
  }
}
```

## Pagination

### Connection Pattern

```typescript
const GetUsersDocument = gql<
  {
    users: {
      edges: Array<{ node: User; cursor: string }>;
      pageInfo: PageInfo;
      totalCount: number;
    };
  },
  { first: number; after?: string }
>`
  query GetUsers($first: Int!, $after: String) {
    users(first: $first, after: $after) {
      edges {
        node {
          id
          name
        }
        cursor
      }
      pageInfo {
        hasNextPage
        endCursor
      }
      totalCount
    }
  }
`;

// Fetch first page
const firstPage = await client.queryTyped(GetUsersDocument, { first: 10 });

// Fetch next page
if (firstPage.ok && firstPage.value.users.pageInfo.hasNextPage) {
  const nextPage = await client.queryTyped(GetUsersDocument, {
    first: 10,
    after: firstPage.value.users.pageInfo.endCursor,
  });
}
```

### Extracting Nodes

```typescript
import { extractNodes } from '@bgql/client';

if (result.ok) {
  const users = extractNodes(result.value.users);
  // users: User[] (extracted from edges)
}
```

## Fragments

### Defining Fragments

```typescript
const UserFieldsFragment = gql`
  fragment UserFields on User {
    id
    name
    email
    avatarUrl
  }
`;

const GetUsersDocument = gql`
  query GetUsers {
    users {
      edges {
        node {
          ...UserFields
        }
      }
    }
  }
  ${UserFieldsFragment}
`;
```

### Type-Safe Fragments

```typescript
import type { FragmentDataOf } from '@bgql/client';

// Get fragment data type
type UserFieldsData = FragmentDataOf<typeof UserFieldsFragment>;

// Use in components
function UserCard({ user }: { user: UserFieldsData }) {
  return <div>{user.name}</div>;
}
```

## Request Options

### AbortController

```typescript
const controller = new AbortController();

// Cancel after 5 seconds
setTimeout(() => controller.abort(), 5000);

const result = await client.query(GetUsers, undefined, {
  signal: controller.signal,
});

if (isErr(result) && result.error.type === 'abort') {
  console.log('Request was cancelled');
}
```

### Custom Headers

```typescript
const result = await client.query(GetUsers, undefined, {
  headers: {
    'X-Request-ID': 'custom-id',
  },
});
```

### Timeout

```typescript
const result = await client.query(GetUsers, undefined, {
  timeout: 5000, // 5 seconds
});
```

## Caching

### Using Cache Middleware

```typescript
import {
  createClient,
  cachingMiddleware,
  createNormalizedCache,
  normalizedCacheMiddleware,
} from '@bgql/client';

// Simple TTL cache
const cache = new Map();
const client = createClient('http://localhost:4000/graphql')
  .use(cachingMiddleware(cache, 60000)); // 60s TTL

// Or normalized cache
const normalizedCache = createNormalizedCache({ ttlMs: 300000 });
const client = createClient('http://localhost:4000/graphql')
  .use(normalizedCacheMiddleware(normalizedCache));
```

### Cache Invalidation

```typescript
// After mutation, update cache
await client.mutate(UpdateUser, { id: '1', input: { name: 'New Name' } });

// Update cache entry
normalizedCache.update('User', '1', { name: 'New Name' });

// Or clear cache
normalizedCache.clear();
```

## Request Deduplication

```typescript
import { createClient, deduplicationMiddleware } from '@bgql/client';

const client = createClient('http://localhost:4000/graphql')
  .use(deduplicationMiddleware());

// These will result in only ONE network request
const [result1, result2, result3] = await Promise.all([
  client.query(GetUser, { id: '1' }),
  client.query(GetUser, { id: '1' }),
  client.query(GetUser, { id: '1' }),
]);
```

## Query Batching

```typescript
import { createClient, batchingMiddleware } from '@bgql/client';

const client = createClient('http://localhost:4000/graphql')
  .use(batchingMiddleware({
    maxBatchSize: 10,
    batchInterval: 10, // ms
  }));

// These will be batched into a single HTTP request
const [users, posts, comments] = await Promise.all([
  client.query(GetUsers),
  client.query(GetPosts),
  client.query(GetComments),
]);
```

## Error Handling

### Network Errors

```typescript
const result = await client.query(GetUsers);

if (isErr(result)) {
  switch (result.error.type) {
    case 'network':
      console.log('Network error:', result.error.message);
      if (result.error.retryable) {
        // Retry the request
      }
      break;
    case 'timeout':
      console.log('Request timed out');
      break;
    case 'abort':
      console.log('Request was cancelled');
      break;
    case 'graphql':
      console.log('GraphQL error:', result.error.message);
      break;
  }
}
```

### GraphQL Errors

```typescript
if (isErr(result) && result.error.type === 'graphql') {
  const { message, locations, path, extensions } = result.error;

  console.error(`GraphQL Error at ${path?.join('.')}: ${message}`);

  if (extensions?.code === 'UNAUTHENTICATED') {
    // Redirect to login
  }
}
```

## Best Practices

### 1. Use Generated Types

```typescript
// ✅ Good: Generated types
import { GetUserDocument } from './generated/graphql';
const result = await client.queryTyped(GetUserDocument, { id: '1' });

// ❌ Avoid: Manual typing
const result = await client.query<{ user: User }>(...);
```

### 2. Handle All Error Cases

```typescript
// ✅ Good: Handle all cases
const result = await client.query(GetUser, { id: '1' });
if (result.ok) {
  matchUnion(result.value.user, {
    User: handleUser,
    NotFoundError: handleNotFound,
    AuthError: handleAuth,
  });
} else {
  handleError(result.error);
}
```

### 3. Use Fragments for Reusability

```typescript
// ✅ Good: Reusable fragments
const UserFields = gql`fragment UserFields on User { id name email }`;
```

## Next Steps

- [Mutations](/frontend/mutations)
- [Type Safety](/frontend/type-safety)
- [Vue.js Integration](/frontend/vue)
