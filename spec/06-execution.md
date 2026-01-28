# Better GraphQL Specification - Execution Model

## 1. Overview

This document describes how Better GraphQL queries are executed, including resolution order, parallelization, error handling, and streaming behavior.

## 2. Execution Phases

### 2.1 Phase Overview

1. **Parsing**: Convert query string to AST
2. **Validation**: Verify query against schema
3. **Execution**: Resolve fields and return data
4. **Serialization**: Convert result to response format

### 2.2 Parsing

The parser converts the query string into an Abstract Syntax Tree (AST):

```graphql
query GetUser($id: ID) {
  user(id: $id) {
    id
    name
  }
}
```

Becomes:
```
Document
└── OperationDefinition (query, "GetUser")
    ├── VariableDefinition ($id: ID)
    └── SelectionSet
        └── Field (user)
            ├── Arguments (id: $id)
            └── SelectionSet
                ├── Field (id)
                └── Field (name)
```

### 2.3 Validation

Validation rules ensure the query is valid:

- All fields exist on their parent types
- All required arguments are provided
- Variable types match expected types
- Fragments are valid and used
- No circular fragment references
- Directives are valid for their locations

## 3. Field Resolution

### 3.1 Resolution Order

Fields are resolved in a specific order:

1. **Root fields**: Resolved first
2. **Nested fields**: Resolved after their parent

```graphql
query {
  user(id: "1") {      # 1. Resolve user
    name               # 2. Resolve name (after user)
    posts {            # 3. Resolve posts (after user)
      title            # 4. Resolve title (after posts)
    }
  }
}
```

### 3.2 Parallel Execution

Sibling fields MAY be resolved in parallel:

```graphql
query {
  users { ... }        # Can run in parallel
  posts { ... }        # Can run in parallel
  comments { ... }     # Can run in parallel
}
```

### 3.3 Serial Execution for Mutations

Mutation root fields MUST be executed serially:

```graphql
mutation {
  createUser(input: {...}) { id }  # Execute first
  updateUser(id: "1", ...) { id }  # Execute after createUser
  deleteUser(id: "2") { success }  # Execute after updateUser
}
```

### 3.4 Resolver Function

Each field has an associated resolver function:

```typescript
interface ResolverContext {
  // Request context
  request: Request;
  headers: Headers;
  cookies: Map<string, string>;

  // Authentication
  user?: User;

  // Data loaders
  loaders: DataLoaders;
}

type Resolver<TParent, TArgs, TResult> = (
  parent: TParent,
  args: TArgs,
  context: ResolverContext,
  info: ResolveInfo
) => TResult | Promise<TResult>;
```

## 4. Batching and DataLoader

### 4.1 N+1 Problem

Without batching:
```graphql
query {
  posts {          # 1 query for posts
    author {       # N queries for authors (one per post)
      name
    }
  }
}
```

### 4.2 DataLoader Pattern

Better GraphQL servers SHOULD use DataLoader for batching:

```typescript
const userLoader = new DataLoader(async (ids: string[]) => {
  const users = await db.users.findMany({ where: { id: { in: ids } } });
  return ids.map(id => users.find(u => u.id === id));
});

// In resolver
const resolver: Resolver<Post, {}, User> = (post, args, ctx) => {
  return ctx.loaders.user.load(post.authorId);
};
```

### 4.3 Batch Window

DataLoader collects requests within a single tick:

```
Tick 1: Collect requests for users [1, 2, 3]
Tick 2: Execute batch query: SELECT * FROM users WHERE id IN (1, 2, 3)
Tick 3: Distribute results to waiting resolvers
```

## 5. Error Handling

### 5.1 Typed Errors (Recommended)

Better GraphQL recommends using union types for expected errors:

```graphql
union UserResult = User | NotFoundError | UnauthorizedError

type Query {
  user(id: ID): UserResult
}
```

Resolver:
```typescript
const userResolver: Resolver<{}, { id: string }, UserResult> = async (
  _,
  { id },
  ctx
) => {
  if (!ctx.user) {
    return { __typename: 'UnauthorizedError', message: 'Not authenticated' };
  }

  const user = await ctx.loaders.user.load(id);
  if (!user) {
    return { __typename: 'NotFoundError', message: 'User not found', resourceId: id };
  }

  return { __typename: 'User', ...user };
};
```

### 5.2 Unexpected Errors

Unexpected errors (exceptions) result in null and an error entry:

```json
{
  "data": {
    "user": null
  },
  "errors": [
    {
      "message": "Internal server error",
      "path": ["user"],
      "extensions": {
        "code": "INTERNAL_ERROR"
      }
    }
  ]
}
```

### 5.3 Error Propagation

For non-nullable fields, errors propagate to the nearest nullable parent:

```graphql
type User {
  id: ID        # Non-nullable
  name: String  # Non-nullable
  email: String # Non-nullable
}

type Query {
  user(id: ID): Option<User>  # Nullable
}
```

If `name` throws an error:
```json
{
  "data": {
    "user": null  // Entire user becomes null
  },
  "errors": [...]
}
```

### 5.4 Partial Success

For nullable fields, errors don't affect siblings:

```graphql
type Query {
  user(id: ID): Option<User>
  posts: Option<List<Post>>
}
```

If `user` fails:
```json
{
  "data": {
    "user": null,
    "posts": [...]  // Still returned
  },
  "errors": [...]
}
```

## 6. Streaming Execution

### 6.1 @defer Execution

Deferred fragments are resolved after the initial response:

```graphql
query {
  user(id: "1") {
    id
    name
    ... @defer(label: "profile") {
      bio
      avatarUrl
    }
  }
}
```

Execution flow:
1. Resolve `id` and `name`
2. Send initial response with `hasNext: true`
3. Resolve `bio` and `avatarUrl`
4. Send incremental response with `hasNext: false`

### 6.2 @stream Execution

Streamed lists send items incrementally:

```graphql
query {
  posts @stream(initialCount: 2) {
    id
    title
  }
}
```

Execution flow:
1. Resolve first 2 posts
2. Send initial response with posts[0..1], `hasNext: true`
3. Resolve remaining posts one by one
4. Send incremental responses for each post
5. Send final response with `hasNext: false`

### 6.3 Priority-based Execution

Higher priority (lower number) deferred fragments are resolved first:

```graphql
query {
  user(id: "1") {
    ... @defer(priority: 1) { avatarUrl }
    ... @defer(priority: 2) { stats { ... } }
    ... @defer(priority: 3) { recommendations { ... } }
  }
}
```

### 6.4 Streaming Response Format

Initial response:
```json
{
  "data": {
    "user": {
      "id": "1",
      "name": "John"
    }
  },
  "hasNext": true
}
```

Incremental response:
```json
{
  "incremental": [
    {
      "path": ["user"],
      "label": "profile",
      "data": {
        "bio": "Developer",
        "avatarUrl": "https://..."
      }
    }
  ],
  "hasNext": false
}
```

## 7. Subscription Execution

### 7.1 Subscription Model

Subscriptions use a pub/sub model:

```typescript
interface SubscriptionResolver<TPayload, TArgs, TResult> {
  subscribe: (
    parent: {},
    args: TArgs,
    context: ResolverContext,
    info: ResolveInfo
  ) => AsyncIterator<TPayload>;

  resolve?: (
    payload: TPayload,
    args: TArgs,
    context: ResolverContext,
    info: ResolveInfo
  ) => TResult;
}
```

### 7.2 Subscription Lifecycle

1. Client sends subscription request
2. Server validates and stores subscription
3. When event occurs, server:
   - Filters subscriptions by event type
   - Resolves the selection set
   - Sends data to client
4. Client or server closes subscription

### 7.3 Subscription Filtering

```graphql
subscription OnPostCreated($authorId: ID) {
  postCreated(authorId: $authorId) {
    id
    title
    author {
      name
    }
  }
}
```

Server filters events by `authorId` before sending.

## 8. Validation Execution

### 8.1 Input Validation

Validation directives are executed during argument coercion:

```graphql
input CreateUserInput {
  email: String @email
  password: String @minLength(8)
  age: Int @range(min: 0, max: 150)
}
```

Validation order:
1. Type coercion (String, Int, etc.)
2. Nullability check
3. Directive validation (@email, @minLength, etc.)

### 8.2 Validation Errors

Validation errors return as typed errors:

```json
{
  "data": {
    "createUser": {
      "__typename": "ValidationError",
      "message": "Invalid email format",
      "field": "email",
      "constraint": "@email"
    }
  }
}
```

## 9. Execution Limits

### 9.1 Query Depth Limit

Servers SHOULD enforce a maximum query depth:

```yaml
execution:
  maxDepth: 10
```

### 9.2 Query Complexity Limit

Servers SHOULD calculate and limit query complexity:

```yaml
execution:
  maxComplexity: 1000
```

### 9.3 Timeout

Servers SHOULD enforce execution timeout:

```yaml
execution:
  timeoutMs: 30000
```

### 9.4 Rate Limiting

Servers SHOULD implement rate limiting:

```yaml
rateLimit:
  requests: 1000
  window: "1h"
  byIP: true
  byUser: true
```

## 10. Execution Context

### 10.1 Request Context

```typescript
interface RequestContext {
  // HTTP
  request: Request;
  response: Response;

  // Authentication
  user?: AuthenticatedUser;
  permissions: string[];

  // Tracing
  requestId: string;
  spanContext?: SpanContext;

  // Data access
  loaders: DataLoaderRegistry;
  db: Database;
  cache: Cache;
}
```

### 10.2 Context Creation

Context is created once per request:

```typescript
const createContext = async (req: Request): Promise<RequestContext> => {
  const token = req.headers.get('Authorization')?.replace('Bearer ', '');
  const user = token ? await verifyToken(token) : undefined;

  return {
    request: req,
    user,
    permissions: user ? await getPermissions(user.id) : [],
    requestId: req.headers.get('X-Request-ID') || crypto.randomUUID(),
    loaders: createLoaders(),
    db: getDatabase(),
    cache: getCache(),
  };
};
```
