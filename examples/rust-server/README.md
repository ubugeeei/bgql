# BGQL Rust Server Example

A comprehensive Better GraphQL server demonstrating all bgql-specific features.

## Features Demonstrated

### Type System
- **Non-null by Default**: `Option<T>` for nullable fields
- **Opaque Types**: `UserId`, `PostId` with compile-time type safety
- **Generics**: `Connection<T>`, `Edge<T>` for pagination
- **Typed Errors**: Union result types (`UserResult = User | NotFoundError`)
- **Input Union**: `input union LoginCredentials` for polymorphic input

### Validation Directives
- `@minLength`, `@maxLength` - String length constraints
- `@min`, `@max` - Numeric range constraints
- `@email`, `@url` - Format validators
- `@pattern(regex)` - Regex pattern matching
- `@trim`, `@lowercase` - Input transformations

### Streaming Directives
- `@defer(label)` - Deferred field resolution
- `@stream(initialCount)` - List streaming
- `@binary(progressive)` - Binary data streaming

### Execution Directives
- `@cache(maxAge, scope)` - Cache control
- `@rateLimit(requests, window)` - Rate limiting
- `@requireAuth(roles)` - Authentication/authorization
- `@priority(level)` - Query priority scheduling
- `@resources(cpu, io)` - Resource hints
- `@resumable(ttl)` - Pause/resume with checkpoints

### Server Fragments
- `@server` - Server-only fragments
- `@island` - Partial hydration for RSC-like patterns

## Project Structure

```
rust-server/
├── schema.bgql        # Full bgql schema demonstrating all features
├── src/
│   ├── main.rs        # HTTP server with resolvers using bgql_sdk
│   └── db.rs          # Mock database with sample data
└── Cargo.toml
```

## Running the Server

```bash
cd examples/rust-server
cargo run --release
```

The server will start on `http://localhost:4000`.

## API Examples

### Query: Get a User (with typed errors)

```bash
curl -s http://localhost:4000/graphql \
  -H "Content-Type: application/json" \
  -d '{
    "query": "{ user(id: \"user_1\") { ... on User { id name email bio posts { id title } } ... on NotFoundError { message code } } }"
  }' | jq
```

### Query: Paginated Users with Connection

```bash
curl -s http://localhost:4000/graphql \
  -H "Content-Type: application/json" \
  -d '{
    "query": "{ users(first: 2) { edges { cursor node { id name } } pageInfo { hasNextPage endCursor } totalCount } }"
  }' | jq
```

### Query: With @defer for Analytics

```bash
curl -s http://localhost:4000/graphql \
  -H "Content-Type: application/json" \
  -H "Accept: multipart/mixed" \
  -d '{
    "query": "{ user(id: \"user_1\") { ... on User { name analytics @defer(label: \"userAnalytics\") { totalPosts totalComments totalLikes } } } }"
  }'
```

### Query: With @stream for Posts

```bash
curl -s http://localhost:4000/graphql \
  -H "Content-Type: application/json" \
  -H "Accept: multipart/mixed" \
  -d '{
    "query": "{ posts @stream(initialCount: 2, label: \"postStream\") { id title } }"
  }'
```

### Mutation: Login with Input Union

```bash
# Email login
curl -s http://localhost:4000/graphql \
  -H "Content-Type: application/json" \
  -d '{
    "query": "mutation { login(credentials: { email: \"alice@example.com\", password: \"password123\" }) { ... on AuthPayload { token user { id name } } ... on InvalidCredentialsError { message } } }"
  }' | jq

# OAuth login
curl -s http://localhost:4000/graphql \
  -H "Content-Type: application/json" \
  -d '{
    "query": "mutation { login(credentials: { provider: GOOGLE, token: \"oauth_token_here\" }) { ... on AuthPayload { token } } }"
  }' | jq
```

### Mutation: Create User with Validation

```bash
curl -s http://localhost:4000/graphql \
  -H "Content-Type: application/json" \
  -d '{
    "query": "mutation { createUser(input: { name: \"Jo\", email: \"invalid\", password: \"123\" }) { ... on User { id name } ... on ValidationError { message field constraint } } }"
  }' | jq
```

Response (validation errors):
```json
{
  "data": {
    "createUser": {
      "__typename": "ValidationError",
      "message": "name: must be at least 3 characters",
      "field": "name",
      "constraint": "3"
    }
  }
}
```

### Authenticated Request

```bash
# First, get a token via login mutation
# Then use it in subsequent requests:
curl -s http://localhost:4000/graphql \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer <token>" \
  -d '{
    "query": "{ me { id name email } }"
  }' | jq
```

## Schema Highlights

### Generic Connection Types

```graphql
type Connection<T extends Node> {
  edges: List<Edge<T>>
  pageInfo: PageInfo
  totalCount: Uint
}

type Edge<T> {
  node: T
  cursor: String
}

# Usage
type Query {
  users(first: Int, after: String): Connection<User>
  posts(first: Int, after: String): Connection<Post>
}
```

### Input Union for Authentication

```graphql
input union LoginCredentials = EmailCredentials | OAuthCredentials

input EmailCredentials {
  email: String @email @trim @lowercase
  password: String @minLength(8)
}

input OAuthCredentials {
  provider: OAuthProvider
  token: String
}
```

### Validation Directives

```graphql
input CreateUserInput {
  name: String @minLength(3) @maxLength(100) @trim
  email: String @email @trim @lowercase
  password: String @minLength(8) @pattern("^(?=.*[A-Z])(?=.*[0-9])")
  bio: Option<String> @maxLength(500) @sanitize
}
```

### Streaming & Deferred Fields

```graphql
type User {
  # Basic fields resolve immediately
  id: UserId
  name: String

  # Analytics can be deferred
  analytics: UserAnalytics @defer(label: "userAnalytics")

  # Posts can be streamed
  posts: List<Post> @stream(initialCount: 5)
}
```

### Server Fragments & Islands

```graphql
# Server-only fragment - not sent to client
fragment UserServerData on User @server {
  passwordHash
  internalNotes
  adminFlags
}

# Island for partial hydration
fragment InteractiveComments on Post @island(
  strategy: VISIBLE
  priority: LOW
) {
  comments @stream {
    id
    content
    author { name }
  }
}
```

## Architecture

### Server Structure

```
rust-server/
├── schema.bgql           # Full BGQL schema with all features
├── src/
│   ├── main.rs          # HTTP server with bgql_sdk resolvers
│   └── db.rs            # In-memory database with sample data
└── Cargo.toml
```

The server uses `bgql_sdk::server::BgqlServer` with inline resolvers for simplicity.
All resolvers are defined in `main.rs` using the builder pattern.

### Railway-Oriented Programming

All operations return `Result<T, E>` types for predictable error handling:

```rust
// Repository method returns Result
pub fn find_user(&self, id: &UserId) -> Result<&User, UserNotFoundError> {
    self.users.get(&id.0)
        .ok_or_else(|| UserNotFoundError::new(id.clone()))
}

// Resolver converts Result to GraphQL union
pub async fn user_resolver(ctx: &Context, args: UserArgs) -> UserResult {
    match ctx.db.find_user(&args.id) {
        Ok(user) => UserResult::User(user.clone()),
        Err(e) => UserResult::NotFoundError(e.to_graphql()),
    }
}
```

### DataLoader Pattern

```rust
// Without DataLoader (N+1 problem):
// query { posts { author { name } } }
// -> 1 query for posts
// -> N queries for each author

// With DataLoader:
// -> 1 query for posts
// -> 1 batched query for all unique authors
```

### Typed Error Handling

```rust
pub async fn user_resolver(ctx: &Context, args: UserArgs) -> UserResult {
    match ctx.db.find_user(&args.id) {
        Some(user) => UserResult::User(user),
        None => UserResult::NotFoundError(NotFoundError {
            message: "User not found".into(),
            code: "USER_NOT_FOUND".into(),
            resource_type: "User".into(),
            resource_id: args.id.0,
        }),
    }
}
```

### Inline Resolver Pattern

```rust
// Using bgql_sdk's BgqlServer builder
let server = BgqlServer::builder()
    .schema_sdl(schema)
    .resolver("Query", "user", move |args, _ctx| {
        let db = db.clone();
        async move {
            let id = args.get("id").and_then(|v| v.as_str()).unwrap_or_default();
            match db.get_user(id).await {
                Some(user) => Ok(serde_json::json!({
                    "__typename": "User",
                    "id": user.id,
                    "name": user.name
                })),
                None => Ok(serde_json::json!({
                    "__typename": "NotFoundError",
                    "message": format!("User '{}' not found", id)
                })),
            }
        }
    })
    .build()?;
```

## Sample Data

**Users:**
- `user_1`: Alice Johnson (alice@example.com) - Admin
- `user_2`: Bob Smith (bob@example.com) - Moderator
- `user_3`: Carol Williams (carol@example.com) - User

**Posts:**
- `post_1`: "Introduction to BGQL" (Published, by Alice)
- `post_2`: "Schema-First Development" (Published, by Alice)
- `post_3`: "DataLoaders Explained" (Published, by Bob)
- `post_4`: "Draft: Typed Errors" (Draft, by Alice)
- `post_5`: "Rust and GraphQL" (Published, by Carol)
