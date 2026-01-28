# Better GraphQL Specification - Go Server SDK

## 1. Overview

The Better GraphQL Go Server SDK provides a high-performance, type-safe GraphQL server implementation with Go's simplicity, strong concurrency support, and excellent tooling.

### 1.1 Core Principles

1. **Schema-first development** - Schema is the source of truth
2. **Type-safe resolvers** - Full type safety from schema to implementation
3. **Go idioms** - Interfaces, error handling, context propagation
4. **Native concurrency** - Goroutines and channels for streaming

### 1.2 Code Generation Flow

```
schema.bgql → bgql codegen → Generated Types + Runtime
                    ↓
              Resolver Implementation
                    ↓
              Type-safe Server
```

## 2. Project Setup

```bash
# Install CLI
go install github.com/better-graphql/bgql@latest

# Generate types from schema
bgql generate --schema ./schema.bgql --output ./generated --target go

# Add dependency
go get github.com/better-graphql/server-go
```

## 3. Generated Types

```go
// generated/types.go

package generated

import "time"

// Newtypes for type safety
type UserId string
type PostId string

// Object types - struct fields are public but immutable by convention
type User struct {
    ID        UserId     `json:"id"`
    Name      string     `json:"name"`
    Email     string     `json:"email"`
    Bio       *string    `json:"bio"`
    AvatarURL *string    `json:"avatarUrl"`
    Role      UserRole   `json:"role"`
    CreatedAt time.Time  `json:"createdAt"`
    UpdatedAt *time.Time `json:"updatedAt"`
}

type Post struct {
    ID          PostId     `json:"id"`
    Title       string     `json:"title"`
    Content     string     `json:"content"`
    AuthorID    UserId     `json:"authorId"`
    Status      PostStatus `json:"status"`
    PublishedAt *time.Time `json:"publishedAt"`
    CreatedAt   time.Time  `json:"createdAt"`
}

// Enums as string types with constants
type UserRole string

const (
    UserRoleAdmin     UserRole = "Admin"
    UserRoleModerator UserRole = "Moderator"
    UserRoleUser      UserRole = "User"
    UserRoleGuest     UserRole = "Guest"
)

type PostStatus string

const (
    PostStatusDraft     PostStatus = "Draft"
    PostStatusPublished PostStatus = "Published"
    PostStatusHidden    PostStatus = "Hidden"
)

// Error types
type NotFoundError struct {
    Message      string `json:"message"`
    Code         string `json:"code"`
    ResourceType string `json:"resourceType"`
    ResourceID   string `json:"resourceId"`
}

func (e NotFoundError) TypeName() string { return "NotFoundError" }
func (e NotFoundError) Error() string    { return e.Message }

type ValidationError struct {
    Message    string `json:"message"`
    Code       string `json:"code"`
    Field      string `json:"field"`
    Constraint string `json:"constraint"`
}

func (e ValidationError) TypeName() string { return "ValidationError" }
func (e ValidationError) Error() string    { return e.Message }

// Result unions using interface with marker method
type UserResult interface {
    isUserResult()
}

func (User) isUserResult()              {}
func (NotFoundError) isUserResult()     {}
func (UnauthorizedError) isUserResult() {}

type CreateUserResult interface {
    isCreateUserResult()
}

func (User) isCreateUserResult()                 {}
func (ValidationError) isCreateUserResult()      {}
func (EmailAlreadyExistsError) isCreateUserResult() {}

// Input types
type CreateUserInput struct {
    Email    string    `json:"email"`
    Password string    `json:"password"`
    Name     string    `json:"name"`
    Role     *UserRole `json:"role,omitempty"`
}

type UpdateUserInput struct {
    Name      *string   `json:"name,omitempty"`
    Bio       *string   `json:"bio,omitempty"`
    AvatarURL *string   `json:"avatarUrl,omitempty"`
}
```

## 4. Resolver Interfaces

```go
// generated/resolvers.go

package generated

import "context"

// Root resolver interface
type Resolvers interface {
    Query() QueryResolver
    Mutation() MutationResolver
    Subscription() SubscriptionResolver
    User() UserFieldResolver
    Post() PostFieldResolver
}

// Query resolver
type QueryResolver interface {
    Me(ctx context.Context) (*User, error)
    User(ctx context.Context, id UserId) (UserResult, error)
    Users(ctx context.Context, args UsersArgs) (*UserConnection, error)
}

type UsersArgs struct {
    First   *int
    After   *string
    Filter  *UserFilter
    OrderBy *UserOrderBy
}

// Mutation resolver
type MutationResolver interface {
    CreateUser(ctx context.Context, input CreateUserInput) (CreateUserResult, error)
    UpdateUser(ctx context.Context, input UpdateUserInput) (UpdateUserResult, error)
    DeleteUser(ctx context.Context, id UserId) (DeleteUserResult, error)
}

// Field resolvers
type UserFieldResolver interface {
    Posts(ctx context.Context, user *User, args PostsArgs) (*PostConnection, error)
    PostsCount(ctx context.Context, user *User) (int, error)
    FollowersCount(ctx context.Context, user *User) (int, error)
    Followers(ctx context.Context, user *User, args FollowersArgs) (*UserConnection, error)
}

type PostFieldResolver interface {
    Author(ctx context.Context, post *Post) (*User, error)
    Comments(ctx context.Context, post *Post, args CommentsArgs) (*CommentConnection, error)
}

// Subscription resolver
type SubscriptionResolver interface {
    PostCreated(ctx context.Context, authorID *UserId) (<-chan *Post, error)
    UserUpdated(ctx context.Context, userID UserId) (<-chan *User, error)
}

// Args types
type PostsArgs struct {
    First *int
    After *string
}

type FollowersArgs struct {
    First *int
    After *string
}
```

## 5. Context and Auth

```go
// pkg/context/context.go

package context

import (
    "context"

    "myapp/generated"
)

type contextKey string

const (
    authKey    contextKey = "auth"
    loadersKey contextKey = "loaders"
)

// Auth holds authentication state
type Auth struct {
    User            *generated.User
    IsAuthenticated bool
}

func (a *Auth) HasRole(role string) bool {
    if a.User == nil {
        return false
    }
    return string(a.User.Role) == role
}

// WithAuth adds auth to context
func WithAuth(ctx context.Context, auth *Auth) context.Context {
    return context.WithValue(ctx, authKey, auth)
}

// AuthFromContext retrieves auth from context
func AuthFromContext(ctx context.Context) *Auth {
    auth, _ := ctx.Value(authKey).(*Auth)
    if auth == nil {
        return &Auth{IsAuthenticated: false}
    }
    return auth
}

// WithLoaders adds DataLoaders to context
func WithLoaders(ctx context.Context, loaders *Loaders) context.Context {
    return context.WithValue(ctx, loadersKey, loaders)
}

// LoadersFromContext retrieves loaders from context
func LoadersFromContext(ctx context.Context) *Loaders {
    loaders, _ := ctx.Value(loadersKey).(*Loaders)
    return loaders
}

// MustAuth returns authenticated context or error
func MustAuth(ctx context.Context) (*Auth, error) {
    auth := AuthFromContext(ctx)
    if !auth.IsAuthenticated {
        return nil, generated.UnauthorizedError{
            Message: "Authentication required",
            Code:    "UNAUTHORIZED",
        }
    }
    return auth, nil
}
```

## 6. Server Implementation

```go
// main.go

package main

import (
    "context"
    "log"
    "net/http"
    "strings"

    bgql "github.com/better-graphql/server-go"
    "myapp/generated"
    "myapp/loaders"
    appctx "myapp/pkg/context"
)

type resolvers struct {
    db      *DB
    loaders *loaders.Loaders
}

func (r *resolvers) Query() generated.QueryResolver {
    return &queryResolver{r}
}

func (r *resolvers) Mutation() generated.MutationResolver {
    return &mutationResolver{r}
}

func (r *resolvers) Subscription() generated.SubscriptionResolver {
    return &subscriptionResolver{r}
}

func (r *resolvers) User() generated.UserFieldResolver {
    return &userFieldResolver{r}
}

type queryResolver struct{ *resolvers }

func (r *queryResolver) Me(ctx context.Context) (*generated.User, error) {
    auth := appctx.AuthFromContext(ctx)
    return auth.User, nil
}

func (r *queryResolver) User(ctx context.Context, id generated.UserId) (generated.UserResult, error) {
    loaders := appctx.LoadersFromContext(ctx)

    user, err := loaders.User.Load(ctx, id)
    if err != nil {
        return nil, err
    }
    if user == nil {
        return generated.NotFoundError{
            Message:      "User not found",
            Code:         "NOT_FOUND",
            ResourceType: "User",
            ResourceID:   string(id),
        }, nil
    }
    return *user, nil
}

func (r *queryResolver) Users(ctx context.Context, args generated.UsersArgs) (*generated.UserConnection, error) {
    first := 10
    if args.First != nil {
        first = *args.First
    }

    return r.db.Users().
        Filter(args.Filter).
        OrderBy(args.OrderBy).
        Paginate(first, args.After)
}

type mutationResolver struct{ *resolvers }

func (r *mutationResolver) CreateUser(
    ctx context.Context,
    input generated.CreateUserInput,
) (generated.CreateUserResult, error) {
    // Check existing
    existing, _ := r.db.Users().FindByEmail(input.Email)
    if existing != nil {
        return generated.EmailAlreadyExistsError{
            Message:       "Email already registered",
            Code:          "EMAIL_EXISTS",
            ExistingEmail: input.Email,
        }, nil
    }

    user, err := r.db.Users().Create(input)
    if err != nil {
        return nil, err
    }
    return *user, nil
}

func (r *mutationResolver) UpdateUser(
    ctx context.Context,
    input generated.UpdateUserInput,
) (generated.UpdateUserResult, error) {
    auth, err := appctx.MustAuth(ctx)
    if err != nil {
        return generated.UnauthorizedError{
            Message: "Authentication required",
            Code:    "UNAUTHORIZED",
        }, nil
    }

    user, err := r.db.Users().Update(auth.User.ID, input)
    if err != nil {
        return nil, err
    }
    return *user, nil
}

type userFieldResolver struct{ *resolvers }

func (r *userFieldResolver) Posts(
    ctx context.Context,
    user *generated.User,
    args generated.PostsArgs,
) (*generated.PostConnection, error) {
    first := 10
    if args.First != nil {
        first = *args.First
    }

    return r.db.Posts().
        ByAuthor(user.ID).
        Paginate(first, args.After)
}

func (r *userFieldResolver) PostsCount(ctx context.Context, user *generated.User) (int, error) {
    loaders := appctx.LoadersFromContext(ctx)
    return loaders.UserPostsCount.Load(ctx, user.ID)
}

type subscriptionResolver struct{ *resolvers }

func (r *subscriptionResolver) PostCreated(
    ctx context.Context,
    authorID *generated.UserId,
) (<-chan *generated.Post, error) {
    ch := make(chan *generated.Post, 1)

    go func() {
        defer close(ch)

        sub := r.pubsub.Subscribe("posts")
        defer sub.Close()

        for {
            select {
            case <-ctx.Done():
                return
            case post := <-sub.Channel():
                // Filter by author if specified
                if authorID != nil && post.AuthorID != *authorID {
                    continue
                }
                ch <- post
            }
        }
    }()

    return ch, nil
}

func main() {
    db := NewDB()
    ldrs := loaders.New(db)

    server := bgql.NewServer(bgql.Config{
        Schema:    "./schema.bgql",
        Resolvers: &resolvers{db: db, loaders: ldrs},
        Context: func(r *http.Request) context.Context {
            ctx := r.Context()

            token := r.Header.Get("Authorization")
            if strings.HasPrefix(token, "Bearer ") {
                token = token[7:]
                if user, err := verifyToken(token); err == nil {
                    ctx = appctx.WithAuth(ctx, &appctx.Auth{
                        User:            user,
                        IsAuthenticated: true,
                    })
                }
            }

            // Add DataLoaders to context
            ctx = appctx.WithLoaders(ctx, ldrs)

            return ctx
        },
    })

    log.Println("Server running on :4000")
    log.Fatal(http.ListenAndServe(":4000", server))
}
```

## 7. DataLoader Implementation

> **Note**: The examples below use GORM, but you can use any ORM or query builder you prefer (Ent, sqlx, sqlc, raw database/sql, etc.).

```go
// loaders/loaders.go

package loaders

import (
    "context"

    "gorm.io/gorm"
    "myapp/generated"
    "myapp/models"
)

type Loaders struct {
    db             *gorm.DB
    User           *UserLoader
    UserPostsCount *UserPostsCountLoader
    UserPosts      *UserPostsLoader
    PostAuthor     *PostAuthorLoader
}

func New(db *gorm.DB) *Loaders {
    return &Loaders{
        db:             db,
        User:           NewUserLoader(db),
        UserPostsCount: NewUserPostsCountLoader(db),
        UserPosts:      NewUserPostsLoader(db),
        PostAuthor:     NewPostAuthorLoader(db),
    }
}

// UserPostsLoader - batch loader for User.posts
type UserPostsLoader struct {
    db *gorm.DB
}

func NewUserPostsLoader(db *gorm.DB) *UserPostsLoader {
    return &UserPostsLoader{db: db}
}

func (l *UserPostsLoader) LoadBatch(
    ctx context.Context,
    userIDs []generated.UserId,
    args generated.PostsArgs,
) (map[generated.UserId][]generated.Post, error) {
    first := 10
    if args.First != nil {
        first = *args.First
    }

    // Convert to string slice for GORM
    ids := make([]string, len(userIDs))
    for i, id := range userIDs {
        ids[i] = string(id)
    }

    // Single query for all users using GORM
    var posts []models.Post
    if err := l.db.WithContext(ctx).
        Where("author_id IN ?", ids).
        Order("created_at DESC").
        Limit(first).
        Find(&posts).Error; err != nil {
        return nil, err
    }

    // Group by author_id
    grouped := make(map[generated.UserId][]generated.Post)
    for _, post := range posts {
        authorID := generated.UserId(post.AuthorID)
        grouped[authorID] = append(grouped[authorID], post.ToGenerated())
    }

    // Ensure all keys have entries (empty slice for users with no posts)
    result := make(map[generated.UserId][]generated.Post)
    for _, id := range userIDs {
        result[id] = grouped[id] // nil becomes empty in JSON
    }

    return result, nil
}

// UserPostsCountLoader - batch loader for User.postsCount
type UserPostsCountLoader struct {
    db *gorm.DB
}

func NewUserPostsCountLoader(db *gorm.DB) *UserPostsCountLoader {
    return &UserPostsCountLoader{db: db}
}

type countResult struct {
    AuthorID string
    Count    int64
}

func (l *UserPostsCountLoader) LoadBatch(
    ctx context.Context,
    userIDs []generated.UserId,
) (map[generated.UserId]int, error) {
    ids := make([]string, len(userIDs))
    for i, id := range userIDs {
        ids[i] = string(id)
    }

    var counts []countResult
    if err := l.db.WithContext(ctx).
        Model(&models.Post{}).
        Select("author_id, COUNT(*) as count").
        Where("author_id IN ?", ids).
        Group("author_id").
        Scan(&counts).Error; err != nil {
        return nil, err
    }

    countMap := make(map[generated.UserId]int)
    for _, c := range counts {
        countMap[generated.UserId(c.AuthorID)] = int(c.Count)
    }

    // Ensure all keys have entries (0 for users with no posts)
    result := make(map[generated.UserId]int)
    for _, id := range userIDs {
        result[id] = countMap[id]
    }

    return result, nil
}

// UserFollowersLoader - batch loader for User.followers
type UserFollowersLoader struct {
    db *gorm.DB
}

func NewUserFollowersLoader(db *gorm.DB) *UserFollowersLoader {
    return &UserFollowersLoader{db: db}
}

func (l *UserFollowersLoader) LoadBatch(
    ctx context.Context,
    userIDs []generated.UserId,
    args generated.FollowersArgs,
) (map[generated.UserId][]generated.User, error) {
    first := 10
    if args.First != nil {
        first = *args.First
    }

    ids := make([]string, len(userIDs))
    for i, id := range userIDs {
        ids[i] = string(id)
    }

    // Query follows with preloaded follower users
    var follows []models.Follow
    if err := l.db.WithContext(ctx).
        Preload("Follower").
        Where("following_id IN ?", ids).
        Limit(first).
        Find(&follows).Error; err != nil {
        return nil, err
    }

    grouped := make(map[generated.UserId][]generated.User)
    for _, f := range follows {
        followingID := generated.UserId(f.FollowingID)
        grouped[followingID] = append(grouped[followingID], f.Follower.ToGenerated())
    }

    result := make(map[generated.UserId][]generated.User)
    for _, id := range userIDs {
        result[id] = grouped[id]
    }

    return result, nil
}
```

## 8. Streaming Support

```go
// Streaming query resolver
func (r *queryResolver) PostsStream(
    ctx context.Context,
    first int,
) (<-chan *generated.Post, error) {
    ch := make(chan *generated.Post, 10)

    go func() {
        defer close(ch)

        cursor := r.db.Posts().Cursor(first)

        for cursor.Next() {
            select {
            case <-ctx.Done():
                return
            case ch <- cursor.Post():
                // Continue streaming
            }
        }
    }()

    return ch, nil
}

// Subscription with filtering
func (r *subscriptionResolver) UserUpdated(
    ctx context.Context,
    userID generated.UserId,
) (<-chan *generated.User, error) {
    ch := make(chan *generated.User, 1)

    go func() {
        defer close(ch)

        sub := r.pubsub.Subscribe("user_updates")
        defer sub.Close()

        for {
            select {
            case <-ctx.Done():
                return
            case update := <-sub.Channel():
                if update.UserID == userID {
                    user, err := r.db.Users().FindByID(userID)
                    if err == nil && user != nil {
                        ch <- user
                    }
                }
            }
        }
    }()

    return ch, nil
}
```

## 9. Error Handling

```go
// pkg/errors/errors.go

package errors

import (
    "fmt"

    "myapp/generated"
)

// NotFound creates a typed not found error
func NotFound(resourceType, resourceID string) generated.NotFoundError {
    return generated.NotFoundError{
        Message:      fmt.Sprintf("%s not found", resourceType),
        Code:         "NOT_FOUND",
        ResourceType: resourceType,
        ResourceID:   resourceID,
    }
}

// Validation creates a typed validation error
func Validation(field, message, constraint string) generated.ValidationError {
    return generated.ValidationError{
        Message:    message,
        Code:       "VALIDATION",
        Field:      field,
        Constraint: constraint,
    }
}

// Unauthorized creates a typed unauthorized error
func Unauthorized(message string) generated.UnauthorizedError {
    if message == "" {
        message = "Authentication required"
    }
    return generated.UnauthorizedError{
        Message: message,
        Code:    "UNAUTHORIZED",
    }
}

// Wrapper for database errors
type DBError struct {
    Op  string
    Err error
}

func (e *DBError) Error() string {
    return fmt.Sprintf("%s: %v", e.Op, e.Err)
}

func (e *DBError) Unwrap() error {
    return e.Err
}
```

## 10. Performance Optimizations

### 10.1 Connection Pooling

```go
import (
    "context"

    "github.com/jackc/pgx/v5/pgxpool"
)

func NewDBPool(ctx context.Context, connString string) (*pgxpool.Pool, error) {
    config, err := pgxpool.ParseConfig(connString)
    if err != nil {
        return nil, err
    }

    config.MaxConns = 16
    config.MinConns = 4

    return pgxpool.NewWithConfig(ctx, config)
}
```

### 10.2 Query Caching

```go
import (
    "time"

    "github.com/patrickmn/go-cache"
)

type CachedResolver struct {
    inner    Resolvers
    cache    *cache.Cache
    duration time.Duration
}

func NewCachedResolver(inner Resolvers, duration time.Duration) *CachedResolver {
    return &CachedResolver{
        inner:    inner,
        cache:    cache.New(duration, duration*2),
        duration: duration,
    }
}

func (r *CachedResolver) User(ctx context.Context, id generated.UserId) (generated.UserResult, error) {
    key := fmt.Sprintf("user:%s", id)

    if cached, found := r.cache.Get(key); found {
        return cached.(generated.UserResult), nil
    }

    result, err := r.inner.Query().User(ctx, id)
    if err != nil {
        return nil, err
    }

    r.cache.Set(key, result, r.duration)
    return result, nil
}
```

## 11. Observability

### 11.1 Tracing

```go
import (
    "go.opentelemetry.io/otel"
    "go.opentelemetry.io/otel/trace"
)

var tracer = otel.Tracer("myapp")

func (r *queryResolver) User(ctx context.Context, id generated.UserId) (generated.UserResult, error) {
    ctx, span := tracer.Start(ctx, "Query.user")
    defer span.End()

    span.SetAttributes(
        attribute.String("user.id", string(id)),
    )

    result, err := r.doGetUser(ctx, id)
    if err != nil {
        span.RecordError(err)
        span.SetStatus(codes.Error, err.Error())
    }

    return result, err
}
```

### 11.2 Metrics

```go
import (
    "github.com/prometheus/client_golang/prometheus"
    "github.com/prometheus/client_golang/prometheus/promauto"
)

var (
    resolverDuration = promauto.NewHistogramVec(prometheus.HistogramOpts{
        Name: "resolver_duration_seconds",
        Help: "Duration of resolver execution",
    }, []string{"resolver"})

    resolverCalls = promauto.NewCounterVec(prometheus.CounterOpts{
        Name: "resolver_calls_total",
        Help: "Total number of resolver calls",
    }, []string{"resolver"})
)

func (r *queryResolver) User(ctx context.Context, id generated.UserId) (generated.UserResult, error) {
    timer := prometheus.NewTimer(resolverDuration.WithLabelValues("Query.user"))
    defer timer.ObserveDuration()

    resolverCalls.WithLabelValues("Query.user").Inc()

    return r.doGetUser(ctx, id)
}
```

### 11.3 Logging

```go
import (
    "go.uber.org/zap"
)

var logger *zap.Logger

func init() {
    logger, _ = zap.NewProduction()
}

func (r *queryResolver) User(ctx context.Context, id generated.UserId) (generated.UserResult, error) {
    logger.Info("fetching user",
        zap.String("user_id", string(id)),
    )

    result, err := r.doGetUser(ctx, id)
    if err != nil {
        logger.Error("failed to fetch user",
            zap.String("user_id", string(id)),
            zap.Error(err),
        )
    }

    return result, err
}
```

## 12. Security

### 12.1 Query Complexity

```go
server := bgql.NewServer(bgql.Config{
    Schema:    "./schema.bgql",
    Resolvers: resolvers,
    Security: bgql.SecurityConfig{
        MaxComplexity: 1000,
        MaxDepth:      10,
        RateLimit: bgql.RateLimitConfig{
            Window:      time.Minute,
            MaxRequests: 100,
        },
    },
})
```

### 12.2 Input Validation

```go
import "github.com/go-playground/validator/v10"

var validate = validator.New()

func (r *mutationResolver) CreateUser(
    ctx context.Context,
    input generated.CreateUserInput,
) (generated.CreateUserResult, error) {
    // Validate input
    if err := validate.Struct(input); err != nil {
        validationErrors := err.(validator.ValidationErrors)
        return generated.ValidationError{
            Message:    validationErrors[0].Error(),
            Code:       "VALIDATION",
            Field:      validationErrors[0].Field(),
            Constraint: validationErrors[0].Tag(),
        }, nil
    }

    // Continue with creation...
}
```

## 13. Testing

```go
package resolvers_test

import (
    "context"
    "testing"

    bgql "github.com/better-graphql/server-go/testing"
    "myapp/generated"
    "myapp/resolvers"
)

func TestGetUser(t *testing.T) {
    client := bgql.NewTestClient(bgql.TestConfig{
        Schema:    "./schema.bgql",
        Resolvers: resolvers.New(mockDB()),
        Context:   mockContext(),
    })

    result, err := client.Query(`
        query GetUser($id: UserId!) {
            user(id: $id) {
                ... on User { id name }
                ... on NotFoundError { message }
            }
        }
    `).
        Var("id", "user_1").
        Execute()

    if err != nil {
        t.Fatalf("query failed: %v", err)
    }

    user, ok := result.Data["user"].(generated.User)
    if !ok {
        t.Fatal("expected User result")
    }

    if user.Name != "John" {
        t.Errorf("expected name John, got %s", user.Name)
    }
}

func TestCreateUser(t *testing.T) {
    client := bgql.NewTestClient(bgql.TestConfig{
        Schema:    "./schema.bgql",
        Resolvers: resolvers.New(mockDB()),
        Context:   authenticatedContext(),
    })

    result, err := client.Mutation(`
        mutation CreateUser($input: CreateUserInput!) {
            createUser(input: $input) {
                ... on User { id email }
                ... on ValidationError { field message }
                ... on EmailAlreadyExistsError { existingEmail }
            }
        }
    `).
        Var("input", map[string]interface{}{
            "email":    "test@example.com",
            "password": "SecurePass123",
            "name":     "Test User",
        }).
        Execute()

    if err != nil {
        t.Fatalf("mutation failed: %v", err)
    }

    user, ok := result.Data["createUser"].(generated.User)
    if !ok {
        t.Fatal("expected User result")
    }

    if user.Email != "test@example.com" {
        t.Errorf("expected email test@example.com, got %s", user.Email)
    }
}

func mockContext() context.Context {
    ctx := context.Background()
    ctx = appctx.WithAuth(ctx, &appctx.Auth{
        User:            mockUser(),
        IsAuthenticated: true,
    })
    ctx = appctx.WithLoaders(ctx, mockLoaders())
    return ctx
}

func mockDB() *DB {
    // Return mock database
    return &DB{}
}
```

## 14. Summary

| Feature | Go SDK |
|---------|--------|
| Schema-first codegen | ✓ |
| Type-safe resolvers | ✓ (interfaces) |
| DataLoader | ✓ |
| @defer/@stream | ✓ (channels) |
| Subscriptions | ✓ (channels) |
| File uploads | ✓ |
| Context propagation | ✓ |
| Tracing | OpenTelemetry |
| Metrics | Prometheus |
| Query complexity | ✓ |
| Input validation | ✓ |
