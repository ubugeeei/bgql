# BGQL Rust Server Example

Type-safe GraphQL server demonstrating codegen-based resolver patterns.

## Features

- **Type-safe Resolvers**: Generated traits (`QueryResolvers`, `MutationResolvers`) with compile-time guarantees
- **Codegen Integration**: Schema-driven type generation for arguments and responses
- **Layered Architecture**: Clean separation of domain, application, infrastructure, and presentation
- **No Manual Cloning**: Server builder extension eliminates repetitive clone boilerplate

## Project Structure

```
rust-server/
├── schema.bgql                  # BGQL schema definition
├── src/
│   ├── main.rs                  # Clean entry point (~50 lines)
│   ├── generated.rs             # Codegen output (types, traits, builder extension)
│   ├── domain/
│   │   ├── mod.rs
│   │   ├── entities.rs          # Value objects (UserId, PostId) and entities
│   │   └── errors.rs            # Domain errors
│   ├── application/
│   │   ├── mod.rs
│   │   └── services.rs          # Business logic (UserService, PostService)
│   ├── infrastructure/
│   │   ├── mod.rs
│   │   └── repositories.rs      # Data access (InMemoryRepository)
│   └── presentation/
│       ├── mod.rs
│       └── resolvers.rs         # Type-safe resolver implementations
└── Cargo.toml
```

## Running

```bash
cd examples/rust-server
cargo run --release
```

Server starts at `http://localhost:4000`.

## Type-Safe Pattern

### 1. Schema (schema.bgql)

```graphql
type Query {
  user(id: UserId): UserResult
  users(pagination: Option<PaginationInput>): Connection<User>
  post(id: PostId): PostResult
  posts(filter: Option<PostFilter>, pagination: Option<PaginationInput>): Connection<Post>
}
```

### 2. Generated Traits (generated.rs)

```rust
#[async_trait]
pub trait QueryResolvers: Send + Sync + 'static {
    async fn user(&self, ctx: &Context, args: UserArgs) -> SdkResult<serde_json::Value>;
    async fn users(&self, ctx: &Context, args: UsersArgs) -> SdkResult<serde_json::Value>;
    // ...
}

pub struct UserArgs {
    pub id: UserId,
}
```

### 3. Implementation (presentation/resolvers.rs)

```rust
pub struct AppQueryResolvers {
    ctx: Arc<AppContext>,
}

#[async_trait]
impl QueryResolvers for AppQueryResolvers {
    async fn user(&self, _ctx: &Context, args: UserArgs) -> SdkResult<serde_json::Value> {
        let id = crate::domain::UserId::new(&args.id.0);
        match self.ctx.user_service.get_user_with_posts(&id).await {
            Ok((user, posts)) => Ok(json!({
                "__typename": "User",
                "id": user.id.0,
                "name": user.name,
                // ...
            })),
            Err(DomainError::UserNotFound(id)) => Ok(json!({
                "__typename": "NotFoundError",
                "message": format!("User '{}' not found", id),
                "code": "NOT_FOUND"
            })),
            Err(e) => Ok(json!({"__typename": "ValidationError", "message": e.to_string()})),
        }
    }
    // ...
}
```

### 4. Clean Entry Point (main.rs)

```rust
#[tokio::main]
async fn main() -> SdkResult<()> {
    // Infrastructure
    let user_repo: Arc<dyn UserRepository> = Arc::new(InMemoryUserRepository::with_seed_data());
    let post_repo: Arc<dyn PostRepository> = Arc::new(InMemoryPostRepository::with_seed_data());
    let comment_repo: Arc<dyn CommentRepository> = Arc::new(InMemoryCommentRepository::new());

    // Application
    let ctx = Arc::new(AppContext::new(user_repo, post_repo, comment_repo));

    // Type-safe resolvers - no manual clone() per resolver!
    let query = Arc::new(AppQueryResolvers::new(ctx.clone()));
    let mutation = Arc::new(AppMutationResolvers::new(ctx));

    BgqlServer::builder()
        .config(ServerConfig::new().port(4000).host("0.0.0.0"))
        .schema_sdl(include_str!("../schema.bgql"))
        .query_resolvers(query)
        .mutation_resolvers(mutation)
        .build()?
        .listen()
        .await
}
```

## API Examples

### Query: Get User

```bash
curl -X POST http://localhost:4000/bgql \
  -H "Content-Type: application/json" \
  -d '{
    "query": "{ user(id: \"user_1\") { ... on User { id name email postsCount } ... on NotFoundError { message } } }"
  }'
```

### Query: List Posts with Filter

```bash
curl -X POST http://localhost:4000/bgql \
  -H "Content-Type: application/json" \
  -d '{
    "query": "{ posts(filter: { status: Published }) { edges { node { id title author { name } } } totalCount } }"
  }'
```

### Mutation: Create Post

```bash
curl -X POST http://localhost:4000/bgql \
  -H "Content-Type: application/json" \
  -d '{
    "query": "mutation { createPost(input: { title: \"New Post\", content: \"This is the content...\" }) { ... on Post { id title } ... on ValidationError { message field } } }"
  }'
```

## Sample Data

**Users:**
- `user_1`: Alice Johnson
- `user_2`: Bob Smith
- `user_3`: Carol Williams

**Posts:**
- `post_1`: "Introduction to BGQL" (Published)
- `post_2`: "Schema-First Development" (Published)
- `post_3`: "Advanced Patterns" (Draft)

## Benefits of Type-Safe Approach

1. **Compile-time Safety**: Missing resolver implementations cause compile errors
2. **IDE Support**: Full autocomplete and type checking for resolver arguments
3. **No Boilerplate**: Server builder extension handles all registration
4. **Layered Architecture**: Clear separation enables testing each layer independently
5. **Schema as Source of Truth**: Types are generated from schema, not hand-written
