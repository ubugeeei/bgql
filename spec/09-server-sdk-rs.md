# Better GraphQL Specification - Rust Server SDK

## 1. Overview

The Better GraphQL Rust Server SDK provides a high-performance, type-safe GraphQL server implementation with zero-cost abstractions, async/await support, and full streaming capabilities.

### 1.1 Core Principles

1. **Schema-first development** - Schema is the source of truth
2. **Type-safe resolvers** - Full type safety from schema to implementation
3. **Zero-cost abstractions** - Compile-time guarantees without runtime overhead
4. **Native async** - First-class tokio/async-std support

### 1.2 Code Generation Flow

```
schema.bgql → bgql codegen → Generated Types + Runtime
                    ↓
              Resolver Implementation
                    ↓
              Type-safe Server
```

## 2. Project Setup

```toml
# Cargo.toml
[dependencies]
better-graphql = "0.1"
better-graphql-macros = "0.1"
tokio = { version = "1", features = ["full"] }
async-trait = "0.1"
serde = { version = "1", features = ["derive"] }
```

```bash
# Generate types from schema
bgql generate --schema ./schema.bgql --output ./src/generated --target rust
```

## 3. Generated Types

```rust
// src/generated/types.rs

use better_graphql::types::*;
use serde::{Deserialize, Serialize};

// Newtypes with type safety
#[derive(Clone, Debug, Serialize, Deserialize, Hash, Eq, PartialEq)]
pub struct UserId(pub String);

#[derive(Clone, Debug, Serialize, Deserialize, Hash, Eq, PartialEq)]
pub struct PostId(pub String);

// Object types - all fields are owned (immutable by default in Rust)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct User {
    pub id: UserId,
    pub name: String,
    pub email: String,
    pub bio: Option<String>,
    pub avatar_url: Option<String>,
    pub role: UserRole,
    pub created_at: DateTime,
    pub updated_at: Option<DateTime>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Post {
    pub id: PostId,
    pub title: String,
    pub content: String,
    pub author_id: UserId,
    pub status: PostStatus,
    pub published_at: Option<DateTime>,
    pub created_at: DateTime,
}

// Enums
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum UserRole {
    Admin,
    Moderator,
    User,
    Guest,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum PostStatus {
    Draft,
    Published,
    Hidden,
}

// Error types
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NotFoundError {
    pub message: String,
    pub code: String,
    pub resource_type: String,
    pub resource_id: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ValidationError {
    pub message: String,
    pub code: String,
    pub field: String,
    pub constraint: String,
}

// Result unions using tagged enum
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "__typename")]
pub enum UserResult {
    User(User),
    NotFoundError(NotFoundError),
    UnauthorizedError(UnauthorizedError),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "__typename")]
pub enum CreateUserResult {
    User(User),
    ValidationError(ValidationError),
    EmailAlreadyExistsError(EmailAlreadyExistsError),
}
```

## 4. Resolver Traits

```rust
// src/generated/resolvers.rs

use async_trait::async_trait;
use crate::generated::types::*;

#[async_trait]
pub trait QueryResolver: Send + Sync {
    async fn me(&self, ctx: &Context) -> Result<Option<User>, Error>;

    async fn user(&self, ctx: &Context, id: UserId) -> Result<UserResult, Error>;

    async fn users(
        &self,
        ctx: &Context,
        first: Option<i32>,
        after: Option<String>,
        filter: Option<UserFilter>,
        order_by: Option<UserOrderBy>,
    ) -> Result<UserConnection, Error>;
}

#[async_trait]
pub trait MutationResolver: Send + Sync {
    async fn create_user(
        &self,
        ctx: &Context,
        input: CreateUserInput,
    ) -> Result<CreateUserResult, Error>;

    async fn update_user(
        &self,
        ctx: &Context,
        input: UpdateUserInput,
    ) -> Result<UpdateUserResult, Error>;
}

#[async_trait]
pub trait UserResolver: Send + Sync {
    async fn posts(
        &self,
        ctx: &Context,
        user: &User,
        first: Option<i32>,
        after: Option<String>,
    ) -> Result<PostConnection, Error>;

    async fn posts_count(&self, ctx: &Context, user: &User) -> Result<u32, Error>;

    async fn followers_count(&self, ctx: &Context, user: &User) -> Result<u32, Error>;
}

pub trait SubscriptionResolver: Send + Sync {
    fn post_created(
        &self,
        ctx: &Context,
        author_id: Option<UserId>,
    ) -> impl Stream<Item = Post> + Send;
}
```

## 5. Context Type

```rust
// src/context.rs

use std::sync::Arc;
use tokio_util::sync::CancellationToken;

pub struct Context {
    pub auth: Auth,
    pub loaders: Arc<DataLoaders>,
    pub signal: CancellationToken,
    pub request: RequestInfo,
}

pub struct Auth {
    pub user: Option<User>,
}

impl Auth {
    pub fn is_authenticated(&self) -> bool {
        self.user.is_some()
    }

    pub fn has_role(&self, role: &str) -> bool {
        self.user
            .as_ref()
            .map(|u| u.roles.contains(&role.to_string()))
            .unwrap_or(false)
    }
}

pub struct RequestInfo {
    pub headers: http::HeaderMap,
    pub cookies: std::collections::HashMap<String, String>,
    pub ip: std::net::IpAddr,
}

// Type-safe authenticated context
pub struct AuthenticatedContext {
    pub user: User,
    pub loaders: Arc<DataLoaders>,
    pub signal: CancellationToken,
}

impl Context {
    pub fn authenticated(&self) -> Option<AuthenticatedContext> {
        self.auth.user.clone().map(|user| AuthenticatedContext {
            user,
            loaders: self.loaders.clone(),
            signal: self.signal.clone(),
        })
    }
}
```

## 6. Server Implementation

```rust
// src/main.rs

use better_graphql::{Server, Context};
use crate::generated::{QueryResolver, MutationResolver, UserResolver};

struct AppResolvers {
    db: DbPool,
    loaders: Arc<DataLoaders>,
}

#[async_trait]
impl QueryResolver for AppResolvers {
    async fn me(&self, ctx: &Context) -> Result<Option<User>, Error> {
        Ok(ctx.auth.user.clone())
    }

    async fn user(&self, ctx: &Context, id: UserId) -> Result<UserResult, Error> {
        match self.loaders.user.load(id.clone()).await {
            Some(user) => Ok(UserResult::User(user)),
            None => Ok(UserResult::NotFoundError(NotFoundError {
                message: "User not found".into(),
                code: "NOT_FOUND".into(),
                resource_type: "User".into(),
                resource_id: id.0,
            })),
        }
    }

    async fn users(
        &self,
        ctx: &Context,
        first: Option<i32>,
        after: Option<String>,
        filter: Option<UserFilter>,
        order_by: Option<UserOrderBy>,
    ) -> Result<UserConnection, Error> {
        let users = self.db.users()
            .filter(filter)
            .order_by(order_by.unwrap_or(UserOrderBy::CreatedAtDesc))
            .paginate(first.unwrap_or(10), after)
            .await?;

        Ok(users)
    }
}

#[async_trait]
impl MutationResolver for AppResolvers {
    async fn create_user(
        &self,
        ctx: &Context,
        input: CreateUserInput,
    ) -> Result<CreateUserResult, Error> {
        // Check existing
        if self.db.users().find_by_email(&input.email).await?.is_some() {
            return Ok(CreateUserResult::EmailAlreadyExistsError(
                EmailAlreadyExistsError {
                    message: "Email already registered".into(),
                    code: "EMAIL_EXISTS".into(),
                    existing_email: input.email,
                }
            ));
        }

        let user = self.db.users().create(input).await?;
        Ok(CreateUserResult::User(user))
    }

    async fn update_user(
        &self,
        ctx: &Context,
        input: UpdateUserInput,
    ) -> Result<UpdateUserResult, Error> {
        // Ensure authenticated
        let auth_ctx = ctx.authenticated()
            .ok_or_else(|| Error::Unauthorized)?;

        let user = self.db.users()
            .update(auth_ctx.user.id.clone(), input)
            .await?;

        Ok(UpdateUserResult::User(user))
    }
}

#[async_trait]
impl UserResolver for AppResolvers {
    async fn posts(
        &self,
        ctx: &Context,
        user: &User,
        first: Option<i32>,
        after: Option<String>,
    ) -> Result<PostConnection, Error> {
        self.db.posts()
            .by_author(&user.id)
            .paginate(first.unwrap_or(10), after)
            .await
    }

    async fn posts_count(&self, ctx: &Context, user: &User) -> Result<u32, Error> {
        self.loaders.user_posts_count.load(user.id.clone()).await
    }

    async fn followers_count(&self, ctx: &Context, user: &User) -> Result<u32, Error> {
        self.loaders.user_followers_count.load(user.id.clone()).await
    }
}

#[tokio::main]
async fn main() {
    let db = create_db_pool().await;
    let loaders = Arc::new(create_data_loaders(db.clone()));

    let resolvers = AppResolvers {
        db,
        loaders: loaders.clone(),
    };

    let server = Server::builder()
        .schema("./schema.bgql")
        .resolvers(resolvers)
        .context(move |req| {
            let loaders = loaders.clone();
            async move {
                let token = req.header("Authorization")
                    .and_then(|h| h.strip_prefix("Bearer "));
                let user = match token {
                    Some(t) => verify_token(t).await.ok(),
                    None => None,
                };

                Context {
                    auth: Auth { user },
                    loaders,
                    signal: req.cancellation_token(),
                    request: RequestInfo {
                        headers: req.headers().clone(),
                        cookies: req.cookies(),
                        ip: req.remote_addr(),
                    },
                }
            }
        })
        .build();

    server.listen("0.0.0.0:4000").await.unwrap();
}
```

## 7. DataLoader Library

The bgql Rust Server SDK provides a built-in DataLoader library for automatic N+1 query prevention with zero-cost abstractions.

### 7.1 Core Concepts

```
┌─────────────────────────────────────────────────────────────┐
│  GraphQL Request                                             │
│  query { users(first: 3) { id, posts(first: 5) { title } } }│
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│  DataLoader Batching                                         │
│  load(user1_id) ─┐                                          │
│  load(user2_id) ─┼─► Single batch call                      │
│  load(user3_id) ─┘                                          │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│  Database Query (1 query instead of N)                      │
│  SELECT * FROM posts WHERE author_id IN ($1, $2, $3)        │
└─────────────────────────────────────────────────────────────┘
```

### 7.2 Library API

```rust
use better_graphql::dataloader::{DataLoader, BatchLoadFn};

/// DataLoader trait
pub trait DataLoader<K, V, A = ()>: Send + Sync {
    /// Load a single value (batched automatically)
    async fn load(&self, key: K, args: A) -> Result<V, Error>;

    /// Load multiple values
    async fn load_many(&self, keys: Vec<K>, args: A) -> Result<HashMap<K, V>, Error>;

    /// Clear cached value for key
    fn clear(&self, key: &K);

    /// Clear all cached values
    fn clear_all(&self);

    /// Prime the cache with a value
    fn prime(&self, key: K, value: V);
}

/// Batch load function trait
#[async_trait]
pub trait BatchLoadFn<K, V, A = ()>: Send + Sync {
    async fn load_batch(
        &self,
        keys: Vec<K>,
        args: A,
        ctx: &Context,
    ) -> Result<HashMap<K, V>, Error>;
}
```

### 7.3 Defining Loaders

> **Note**: The examples below use SeaORM, but you can use any ORM or query builder you prefer (Diesel, sqlx, cornucopia, etc.).

```rust
// src/loaders.rs

use crate::generated::loaders::*;
use crate::entities::{post, user, follow};
use sea_orm::{prelude::*, QueryOrder, QuerySelect};
use std::collections::HashMap;
use async_trait::async_trait;

pub struct AppLoaders {
    db: DatabaseConnection,
}

impl AppLoaders {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

// Simple key-value loader (no args)
#[async_trait]
impl UserLoader for AppLoaders {
    async fn load_batch(
        &self,
        user_ids: Vec<UserId>,
        _args: &(),
        _ctx: &Context,
    ) -> Result<HashMap<UserId, Option<User>>, Error> {
        let ids: Vec<String> = user_ids.iter().map(|id| id.0.clone()).collect();

        let users = user::Entity::find()
            .filter(user::Column::Id.is_in(&ids))
            .all(&self.db)
            .await?;

        let user_map: HashMap<_, _> = users.into_iter()
            .map(|u| (UserId(u.id.clone()), Some(u.into())))
            .collect();

        Ok(user_ids.into_iter()
            .map(|id| (id.clone(), user_map.get(&id).cloned().flatten()))
            .collect())
    }
}

// Loader with field arguments
#[async_trait]
impl UserPostsLoader for AppLoaders {
    async fn load_batch(
        &self,
        user_ids: Vec<UserId>,
        args: &PostsArgs,
        _ctx: &Context,
    ) -> Result<HashMap<UserId, Vec<Post>>, Error> {
        let ids: Vec<String> = user_ids.iter().map(|id| id.0.clone()).collect();

        let posts = post::Entity::find()
            .filter(post::Column::AuthorId.is_in(&ids))
            .order_by_desc(post::Column::CreatedAt)
            .limit(args.first.unwrap_or(10) as u64)
            .all(&self.db)
            .await?;

        // Group by author_id
        let mut grouped: HashMap<UserId, Vec<Post>> = HashMap::new();
        for post in posts {
            let author_id = UserId(post.author_id.clone());
            grouped.entry(author_id)
                .or_default()
                .push(post.into());
        }

        // Ensure all keys have entries
        Ok(user_ids.into_iter()
            .map(|id| (id.clone(), grouped.remove(&id).unwrap_or_default()))
            .collect())
    }
}

// Aggregation loader
#[async_trait]
impl UserPostsCountLoader for AppLoaders {
    async fn load_batch(
        &self,
        user_ids: Vec<UserId>,
        _args: &(),
        _ctx: &Context,
    ) -> Result<HashMap<UserId, u32>, Error> {
        let ids: Vec<String> = user_ids.iter().map(|id| id.0.clone()).collect();

        #[derive(FromQueryResult)]
        struct CountResult {
            author_id: String,
            count: i64,
        }

        let counts: Vec<CountResult> = post::Entity::find()
            .select_only()
            .column(post::Column::AuthorId)
            .column_as(post::Column::Id.count(), "count")
            .filter(post::Column::AuthorId.is_in(&ids))
            .group_by(post::Column::AuthorId)
            .into_model::<CountResult>()
            .all(&self.db)
            .await?;

        let count_map: HashMap<_, _> = counts.into_iter()
            .map(|r| (UserId(r.author_id), r.count as u32))
            .collect();

        Ok(user_ids.into_iter()
            .map(|id| (id.clone(), *count_map.get(&id).unwrap_or(&0)))
            .collect())
    }
}

// One-to-many relation loader
#[async_trait]
impl UserFollowersLoader for AppLoaders {
    async fn load_batch(
        &self,
        user_ids: Vec<UserId>,
        args: &FollowersArgs,
        _ctx: &Context,
    ) -> Result<HashMap<UserId, Vec<User>>, Error> {
        let ids: Vec<String> = user_ids.iter().map(|id| id.0.clone()).collect();

        let follows_with_users = follow::Entity::find()
            .filter(follow::Column::FollowingId.is_in(&ids))
            .find_also_related(user::Entity)
            .limit(args.first.unwrap_or(10) as u64)
            .all(&self.db)
            .await?;

        let mut grouped: HashMap<UserId, Vec<User>> = HashMap::new();
        for (follow, maybe_user) in follows_with_users {
            if let Some(user) = maybe_user {
                let following_id = UserId(follow.following_id);
                grouped.entry(following_id)
                    .or_default()
                    .push(user.into());
            }
        }

        Ok(user_ids.into_iter()
            .map(|id| (id.clone(), grouped.remove(&id).unwrap_or_default()))
            .collect())
    }
}
```

### 7.4 Using Loaders in Resolvers

```rust
#[async_trait]
impl QueryResolver for AppResolvers {
    async fn user(&self, ctx: &Context, id: UserId) -> Result<UserResult, Error> {
        // Single load - batched with other loads in same tick
        match ctx.loaders.user.load(id.clone(), ()).await? {
            Some(user) => Ok(UserResult::User(user)),
            None => Ok(UserResult::NotFoundError(NotFoundError {
                message: "User not found".into(),
                resource_type: "User".into(),
                resource_id: id.0,
                code: "NOT_FOUND".into(),
            })),
        }
    }
}

#[async_trait]
impl UserResolver for AppResolvers {
    async fn posts(
        &self,
        ctx: &Context,
        user: &User,
        first: Option<i32>,
        after: Option<String>,
    ) -> Result<Vec<Post>, Error> {
        // Automatically batched across all User instances
        ctx.loaders.user_posts.load(user.id.clone(), PostsArgs { first, after }).await
    }

    async fn posts_count(&self, ctx: &Context, user: &User) -> Result<u32, Error> {
        ctx.loaders.user_posts_count.load(user.id.clone(), ()).await
    }
}
```

### 7.5 Advanced Patterns

#### Cache Priming

```rust
#[async_trait]
impl QueryResolver for AppResolvers {
    async fn users(
        &self,
        ctx: &Context,
        first: Option<i32>,
        _after: Option<String>,
        _filter: Option<UserFilter>,
        _order_by: Option<UserOrderBy>,
    ) -> Result<Vec<User>, Error> {
        let users = self.db.users()
            .limit(first.unwrap_or(10))
            .all()
            .await?;

        // Prime the cache for subsequent user(id) calls
        for user in &users {
            ctx.loaders.user.prime(user.id.clone(), Some(user.clone()));
        }

        Ok(users)
    }
}
```

#### Cache Invalidation

```rust
#[async_trait]
impl MutationResolver for AppResolvers {
    async fn update_user(
        &self,
        ctx: &Context,
        input: UpdateUserInput,
    ) -> Result<UpdateUserResult, Error> {
        let user = self.db.users()
            .update(input.id.clone(), input)
            .await?;

        // Clear stale cache entry
        ctx.loaders.user.clear(&input.id);

        // Or prime with new value
        ctx.loaders.user.prime(input.id, Some(user.clone()));

        Ok(UpdateUserResult::User(user))
    }
}
```

### 7.6 Generated Types

```rust
// generated/loaders.rs

use async_trait::async_trait;
use std::collections::HashMap;

/// User loader - loads users by ID
#[async_trait]
pub trait UserLoader: Send + Sync {
    async fn load_batch(
        &self,
        keys: Vec<UserId>,
        args: &(),
        ctx: &Context,
    ) -> Result<HashMap<UserId, Option<User>>, Error>;
}

/// UserPosts loader - loads posts for users with pagination
#[async_trait]
pub trait UserPostsLoader: Send + Sync {
    async fn load_batch(
        &self,
        keys: Vec<UserId>,
        args: &PostsArgs,
        ctx: &Context,
    ) -> Result<HashMap<UserId, Vec<Post>>, Error>;
}

#[derive(Clone, Debug)]
pub struct PostsArgs {
    pub first: Option<i32>,
    pub after: Option<String>,
}

/// DataLoaders container
pub struct DataLoaders {
    pub user: Box<dyn UserLoader>,
    pub user_posts: Box<dyn UserPostsLoader>,
    pub user_posts_count: Box<dyn UserPostsCountLoader>,
    pub user_followers: Box<dyn UserFollowersLoader>,
}
```

### 7.7 Performance Considerations

| Pattern | Without DataLoader | With DataLoader |
|---------|-------------------|-----------------|
| `users(first: 10) { posts }` | 1 + 10 = 11 queries | 1 + 1 = 2 queries |
| `users(first: 100) { posts, comments }` | 1 + 200 = 201 queries | 1 + 2 = 3 queries |
| Nested relations (3 levels) | 1 + N + N² queries | 1 + 1 + 1 = 3 queries |

**Best practices:**
- Always use loaders for field resolvers on list types
- Prime cache after creating/updating entities
- Clear cache after mutations that affect related entities
- Use `load_many` when you have multiple keys upfront
```

## 8. Streaming Support

```rust
// @defer support with async streams
impl UserResolver for AppResolvers {
    // Returns a Deferred that resolves lazily
    async fn posts(
        &self,
        ctx: &Context,
        user: &User,
        first: Option<i32>,
        after: Option<String>,
    ) -> Result<PostConnection, Error> {
        // Check cancellation
        if ctx.signal.is_cancelled() {
            return Err(Error::Aborted);
        }

        self.db.posts()
            .by_author(&user.id)
            .paginate(first.unwrap_or(10), after)
            .await
    }
}

// @stream support with async generators
impl QueryResolver for AppResolvers {
    fn posts_stream(
        &self,
        ctx: &Context,
        first: i32,
    ) -> impl Stream<Item = Post> + Send + '_ {
        async_stream::stream! {
            let mut cursor = self.db.posts().cursor(first);

            while let Some(batch) = cursor.next().await {
                for post in batch {
                    if ctx.signal.is_cancelled() {
                        return;
                    }
                    yield post;
                }
            }
        }
    }
}

// Subscription implementation
impl SubscriptionResolver for AppResolvers {
    fn post_created(
        &self,
        ctx: &Context,
        author_id: Option<UserId>,
    ) -> impl Stream<Item = Post> + Send + '_ {
        async_stream::stream! {
            let mut rx = self.pubsub.subscribe::<Post>("posts");

            while let Some(post) = rx.recv().await {
                if ctx.signal.is_cancelled() {
                    break;
                }

                // Filter by author if specified
                if let Some(ref id) = author_id {
                    if &post.author_id != id {
                        continue;
                    }
                }

                yield post;
            }
        }
    }
}
```

## 9. Error Handling

```rust
// Type-safe error handling with thiserror
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ResolverError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Unauthorized")]
    Unauthorized,

    #[error("Not found: {resource_type} with id {resource_id}")]
    NotFound {
        resource_type: &'static str,
        resource_id: String,
    },

    #[error("Validation failed: {field} - {message}")]
    Validation {
        field: String,
        message: String,
    },

    #[error("Request aborted")]
    Aborted,
}

// Convert to GraphQL error types
impl From<ResolverError> for UserResult {
    fn from(err: ResolverError) -> Self {
        match err {
            ResolverError::NotFound { resource_type, resource_id } => {
                UserResult::NotFoundError(NotFoundError {
                    message: format!("{} not found", resource_type),
                    code: "NOT_FOUND".into(),
                    resource_type: resource_type.into(),
                    resource_id,
                })
            }
            ResolverError::Unauthorized => {
                UserResult::UnauthorizedError(UnauthorizedError {
                    message: "Authentication required".into(),
                    code: "UNAUTHORIZED".into(),
                })
            }
            _ => panic!("Unexpected error type"),
        }
    }
}
```

## 10. Performance Optimizations

### 10.1 Connection Pooling

```rust
use deadpool_postgres::{Pool, Manager, Runtime};

pub async fn create_db_pool() -> Pool {
    let config = tokio_postgres::Config::from_str(&std::env::var("DATABASE_URL").unwrap())
        .unwrap();

    let manager = Manager::new(config, tokio_postgres::NoTls);

    Pool::builder(manager)
        .max_size(16)
        .runtime(Runtime::Tokio1)
        .build()
        .unwrap()
}
```

### 10.2 Query Caching

```rust
use moka::future::Cache;
use std::time::Duration;

pub struct CachedResolver<R> {
    inner: R,
    cache: Cache<String, serde_json::Value>,
}

impl<R> CachedResolver<R> {
    pub fn new(inner: R) -> Self {
        let cache = Cache::builder()
            .max_capacity(10_000)
            .time_to_live(Duration::from_secs(60))
            .build();

        Self { inner, cache }
    }
}
```

## 11. Observability

### 11.1 Tracing

```rust
use tracing::{instrument, info, error};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(tracing_opentelemetry::layer())
        .init();

    // ... server setup
}

// Instrumented resolver
#[async_trait]
impl QueryResolver for AppResolvers {
    #[instrument(skip(self, ctx))]
    async fn user(&self, ctx: &Context, id: UserId) -> Result<UserResult, Error> {
        info!(user_id = %id.0, "Fetching user");

        match self.loaders.user.load(id.clone()).await {
            Some(user) => {
                info!(user_id = %id.0, "User found");
                Ok(UserResult::User(user))
            }
            None => {
                info!(user_id = %id.0, "User not found");
                Ok(UserResult::NotFoundError(NotFoundError {
                    message: "User not found".into(),
                    code: "NOT_FOUND".into(),
                    resource_type: "User".into(),
                    resource_id: id.0,
                }))
            }
        }
    }
}
```

### 11.2 Metrics

```rust
use metrics::{counter, histogram};
use metrics_exporter_prometheus::PrometheusBuilder;

fn setup_metrics() {
    PrometheusBuilder::new()
        .with_http_listener(([0, 0, 0, 0], 9090))
        .install()
        .unwrap();
}

// In resolvers
async fn user(&self, ctx: &Context, id: UserId) -> Result<UserResult, Error> {
    let start = std::time::Instant::now();

    let result = self.loaders.user.load(id.clone()).await;

    histogram!("resolver_duration_seconds", start.elapsed().as_secs_f64(),
        "resolver" => "Query.user");

    counter!("resolver_calls_total", 1, "resolver" => "Query.user");

    // ... rest of resolver
}
```

## 12. Security

### 12.1 Query Complexity

```rust
let server = Server::builder()
    .schema("./schema.bgql")
    .resolvers(resolvers)
    .security(SecurityConfig {
        max_complexity: 1000,
        max_depth: 10,
        rate_limit: RateLimitConfig {
            window: Duration::from_secs(60),
            max_requests: 100,
        },
    })
    .build();
```

### 12.2 Input Validation

```rust
// Automatic validation from schema directives
// Custom validation with validator crate
use validator::Validate;

#[derive(Debug, Validate, Deserialize)]
pub struct CreateUserInput {
    #[validate(email)]
    pub email: String,

    #[validate(length(min = 8))]
    pub password: String,

    #[validate(length(min = 1, max = 100))]
    pub name: String,
}
```

## 13. Testing

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use better_graphql::testing::TestClient;

    #[tokio::test]
    async fn test_get_user() {
        let client = TestClient::new(resolvers, mock_context());

        let result = client.query(r#"
            query GetUser($id: UserId) {
                user(id: $id) {
                    ... on User { id name }
                    ... on NotFoundError { message }
                }
            }
        "#)
        .var("id", "user_1")
        .execute()
        .await;

        assert!(matches!(result.data.user, UserResult::User(_)));
    }

    #[tokio::test]
    async fn test_create_user() {
        let client = TestClient::new(resolvers, mock_context());

        let result = client.mutation(r#"
            mutation CreateUser($input: CreateUserInput!) {
                createUser(input: $input) {
                    ... on User { id email }
                    ... on ValidationError { field message }
                }
            }
        "#)
        .var("input", json!({
            "email": "test@example.com",
            "password": "SecurePass123",
            "name": "Test User"
        }))
        .execute()
        .await;

        assert!(matches!(result.data.create_user, CreateUserResult::User(_)));
    }

    fn mock_context() -> Context {
        Context {
            auth: Auth { user: Some(mock_user()) },
            loaders: Arc::new(mock_loaders()),
            signal: CancellationToken::new(),
            request: mock_request_info(),
        }
    }
}
```
