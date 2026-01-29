# Directives

Directives modify the behavior of fields, types, and arguments.

## Built-in Directives

### @deprecated

Mark fields or types as deprecated:

```graphql
type User {
  id: ID
  name: String
  username: String @deprecated(reason: "Use name instead")
  createdAt: DateTime
  createdTime: DateTime @deprecated(reason: "Use createdAt instead")
}

enum Status {
  ACTIVE
  INACTIVE
  PENDING @deprecated(reason: "Use INACTIVE instead")
}
```

### @specifiedBy

Document custom scalars:

```graphql
scalar Email @specifiedBy(url: "https://html.spec.whatwg.org/#valid-e-mail-address")
scalar URL @specifiedBy(url: "https://url.spec.whatwg.org/")
scalar UUID @specifiedBy(url: "https://tools.ietf.org/html/rfc4122")
```

## Validation Directives

### String Validation

```graphql
input CreateUserInput {
  # Length constraints
  name: String @minLength(1) @maxLength(100)
  bio: Option<String> @maxLength(500)

  # Format validation
  email: String @email
  website: Option<String> @url
  handle: String @pattern(regex: "^[a-z0-9_]+$")

  # UUID format
  externalId: Option<String> @uuid
}
```

### Numeric Validation

```graphql
input CreateProductInput {
  name: String
  price: Float @min(0)
  quantity: Int @min(0) @max(1000)
  discount: Option<Float> @min(0) @max(100)
  rating: Option<Float> @min(0) @max(5)
}
```

### List Validation

```graphql
input CreatePostInput {
  title: String
  content: String
  tags: List<String> @minLength(1) @maxLength(10)  # 1-10 tags
  images: Option<List<String>> @maxLength(20)      # Max 20 images
}
```

## Authorization Directives

### @requireAuth

Require authentication:

```graphql
type Query {
  publicPosts: List<Post>
  myProfile: User @requireAuth
  myPosts: List<Post> @requireAuth
}

type Mutation {
  createPost(input: CreatePostInput): Post @requireAuth
}
```

### @hasRole

Require specific role:

```graphql
type Query {
  adminDashboard: AdminStats @hasRole(role: ADMIN)
  moderatorQueue: List<Report> @hasRole(role: MODERATOR)
}

type Mutation {
  banUser(id: ID): User @hasRole(role: ADMIN)
  deletePost(id: ID): Boolean @hasRole(role: MODERATOR)
}
```

### @hasPermission

Require specific permission:

```graphql
type Query {
  users: List<User> @hasPermission(permission: "users:read")
  auditLogs: List<AuditLog> @hasPermission(permission: "audit:read")
}

type Mutation {
  updateUser(id: ID, input: UpdateUserInput): User
    @hasPermission(permission: "users:write")
}
```

## Caching Directives

### @cacheControl

Set cache hints:

```graphql
type Query {
  # Cache for 1 hour, shared cache
  publicConfig: Config @cacheControl(maxAge: 3600, scope: PUBLIC)

  # Cache for 5 minutes, private to user
  myNotifications: List<Notification>
    @cacheControl(maxAge: 300, scope: PRIVATE)

  # No caching
  currentTime: DateTime @cacheControl(maxAge: 0)
}

type User {
  id: ID
  name: String @cacheControl(maxAge: 3600)
  email: String @cacheControl(maxAge: 0, scope: PRIVATE)
}
```

## Rate Limiting Directives

### @rateLimit

Apply rate limits:

```graphql
type Mutation {
  # 10 requests per minute
  login(email: String, password: String): AuthResult
    @rateLimit(requests: 10, window: "1m")

  # 100 requests per hour
  sendMessage(input: MessageInput): Message
    @rateLimit(requests: 100, window: "1h")

  # 1000 requests per day
  createPost(input: PostInput): Post
    @rateLimit(requests: 1000, window: "1d")
}
```

### @complexity

Set query complexity:

```graphql
type Query {
  # Simple query, low complexity
  user(id: ID): User @complexity(value: 1)

  # Paginated query, medium complexity
  users(first: Int): Connection<User> @complexity(value: 10)

  # Expensive operation
  search(query: String): List<SearchResult> @complexity(value: 50)
}
```

## Streaming Directives

### @defer

Defer field resolution:

```graphql
query GetUserProfile($id: ID!) {
  user(id: $id) {
    id
    name

    # Deferred - arrives later
    ... @defer(label: "posts") {
      posts {
        id
        title
      }
    }

    # Also deferred
    ... @defer(label: "followers") {
      followers {
        id
        name
      }
    }
  }
}
```

### @stream

Stream list items:

```graphql
query GetFeed {
  feed @stream(initialCount: 10, label: "feed") {
    id
    content
    author {
      name
    }
  }
}
```

## Schema Directives

### @internal

Hide fields from public schema:

```graphql
type User {
  id: ID
  name: String
  email: String
  passwordHash: String @internal  # Not exposed in queries
  internalNotes: String @internal
}
```

### @external

Mark field as external (federation):

```graphql
type Product @key(fields: "id") {
  id: ID @external
  name: String
  reviews: List<Review>
}
```

## Custom Directives

Define your own directives:

```graphql
directive @log(level: LogLevel = INFO) on FIELD_DEFINITION

enum LogLevel {
  DEBUG
  INFO
  WARN
  ERROR
}

type Mutation {
  createUser(input: CreateUserInput): User @log(level: INFO)
  deleteUser(id: ID): Boolean @log(level: WARN)
}
```

Implement in resolver:

```typescript
import { withDirective } from '@bgql/server';

const logDirective = withDirective('log', async (next, args, context) => {
  const { level } = args;
  const start = Date.now();

  try {
    const result = await next();
    context.logger[level.toLowerCase()](
      `Operation completed in ${Date.now() - start}ms`
    );
    return result;
  } catch (error) {
    context.logger.error(`Operation failed: ${error.message}`);
    throw error;
  }
});
```

## Directive Locations

Directives can be placed on:

| Location | Example |
|----------|---------|
| `FIELD_DEFINITION` | `name: String @deprecated` |
| `ARGUMENT_DEFINITION` | `user(id: ID @auth): User` |
| `INPUT_FIELD_DEFINITION` | `email: String @email` |
| `OBJECT` | `type User @cacheControl(maxAge: 60)` |
| `INTERFACE` | `interface Node @key(fields: "id")` |
| `UNION` | `union Result @tag(name: "public")` |
| `ENUM` | `enum Status @deprecated` |
| `ENUM_VALUE` | `PENDING @deprecated` |
| `INPUT_OBJECT` | `input UserInput @validate` |
| `SCALAR` | `scalar Email @specifiedBy(...)` |

## Best Practices

### 1. Use Validation Directives

```graphql
# ✅ Good: Validate at schema level
input CreateUserInput {
  email: String @email
  age: Int @min(0) @max(150)
}

# ❌ Avoid: No validation
input CreateUserInput {
  email: String
  age: Int
}
```

### 2. Apply Auth Directives Consistently

```graphql
# ✅ Good: Clear authorization
type Query {
  publicPosts: List<Post>
  myPosts: List<Post> @requireAuth
  allUsers: List<User> @hasRole(role: ADMIN)
}
```

### 3. Document Custom Directives

```graphql
"""
Logs field resolution with the specified level.
@param level - The log level (DEBUG, INFO, WARN, ERROR)
"""
directive @log(level: LogLevel = INFO) on FIELD_DEFINITION
```

### 4. Use Cache Hints Appropriately

```graphql
# ✅ Good: Appropriate caching
type Query {
  # Public, rarely changes
  siteConfig: Config @cacheControl(maxAge: 86400, scope: PUBLIC)

  # User-specific, changes often
  notifications: List<Notification> @cacheControl(maxAge: 60, scope: PRIVATE)

  # Real-time data
  onlineUsers: Int @cacheControl(maxAge: 0)
}
```

## Next Steps

- [Module System](/schema/modules)
- [Types](/schema/types)
- [Generics](/schema/generics)
