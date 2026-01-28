# Better GraphQL Specification - TypeScript Server SDK

## 1. Overview

The Better GraphQL TypeScript Server SDK provides a type-safe, high-performance GraphQL server implementation with full support for streaming, DataLoader, and schema-first development.

### 1.1 Core Principles

1. **Schema-first development** - Schema is the source of truth
2. **Type-safe resolvers** - Full type safety from schema to implementation
3. **High performance** - Optimized execution, DataLoader, caching
4. **Native streaming** - First-class @defer, @stream, subscriptions

### 1.2 Code Generation Flow

```
schema.bgql → bgql codegen → Generated Types + Runtime
                    ↓
              Resolver Implementation
                    ↓
              Type-safe Server
```

### 1.3 Generated Artifacts

| Artifact | Description |
|----------|-------------|
| Type definitions | All types, inputs, enums, unions |
| Resolver interfaces | Type-safe resolver signatures |
| Context type | Request context with auth, headers, etc. |
| DataLoader types | Batch loading interfaces |
| Validation | Input validation from directives |
| Error types | Typed error constructors |

## 2. Project Setup

```bash
# Install
npm install @better-graphql/server

# Generate types from schema
npx bgql generate --schema ./schema.bgql --output ./generated
```

## 3. Generated Types

```typescript
// generated/types.ts

// Scalar types
export type UserId = string & { readonly __brand: "UserId" };
export type PostId = string & { readonly __brand: "PostId" };

// Object types - all properties readonly by default
export interface User {
  readonly id: UserId;
  readonly name: string;
  readonly email: string;
  readonly bio: string | null;
  readonly avatarUrl: string | null;
  readonly role: UserRole;
  readonly createdAt: Date;
  readonly updatedAt: Date | null;
}

export interface Post {
  readonly id: PostId;
  readonly title: string;
  readonly content: string;
  readonly authorId: UserId;
  readonly status: PostStatus;
  readonly publishedAt: Date | null;
  readonly createdAt: Date;
}

// Enums (PascalCase)
export type UserRole = "Admin" | "Moderator" | "User" | "Guest";
export type PostStatus = "Draft" | "Published" | "Hidden";

// Input types - readonly for immutability
export interface CreateUserInput {
  readonly email: string;
  readonly password: string;
  readonly name: string;
  readonly role?: UserRole;
}

// Error types - readonly by default
export interface NotFoundError {
  readonly __typename: "NotFoundError";
  readonly message: string;
  readonly code: string;
  readonly resourceType: string;
  readonly resourceId: string;
}

export interface ValidationError {
  readonly __typename: "ValidationError";
  readonly message: string;
  readonly code: string;
  readonly field: string;
  readonly constraint: string;
}

// Result unions
export type UserResult = User | NotFoundError | UnauthorizedError;
export type CreateUserResult = User | ValidationError | EmailAlreadyExistsError;
```

## 4. Resolver Interface

```typescript
// generated/resolvers.ts

export interface Resolvers<TContext extends BaseContext = BaseContext> {
  Query: QueryResolvers<TContext>;
  Mutation: MutationResolvers<TContext>;
  Subscription: SubscriptionResolvers<TContext>;
  User: UserResolvers<TContext>;
  Post: PostResolvers<TContext>;
}

export interface QueryResolvers<TContext> {
  me: (
    parent: {},
    args: {},
    context: TContext,
    info: ResolveInfo
  ) => Promise<User | null> | User | null;

  user: (
    parent: {},
    args: { readonly id: UserId },
    context: TContext,
    info: ResolveInfo
  ) => Promise<UserResult> | UserResult;

  users: (
    parent: {},
    args: {
      readonly first?: number;
      readonly after?: string | null;
      readonly filter?: UserFilter | null;
      readonly orderBy?: UserOrderBy;
    },
    context: TContext,
    info: ResolveInfo
  ) => Promise<UserConnection> | UserConnection;
}

export interface MutationResolvers<TContext> {
  createUser: (
    parent: {},
    args: { readonly input: CreateUserInput },
    context: TContext,
    info: ResolveInfo
  ) => Promise<CreateUserResult> | CreateUserResult;

  updateUser: (
    parent: {},
    args: { readonly input: UpdateUserInput },
    context: TContext,
    info: ResolveInfo
  ) => Promise<UpdateUserResult> | UpdateUserResult;
}

// Field resolvers for computed fields
export interface UserResolvers<TContext> {
  // Computed fields
  posts?: (
    parent: User,
    args: { readonly first?: number; readonly after?: string | null },
    context: TContext,
    info: ResolveInfo
  ) => Promise<PostConnection> | PostConnection;

  postsCount?: (
    parent: User,
    args: {},
    context: TContext,
    info: ResolveInfo
  ) => Promise<number> | number;

  followersCount?: (
    parent: User,
    args: {},
    context: TContext,
    info: ResolveInfo
  ) => Promise<number> | number;
}
```

## 5. Context Type

```typescript
// generated/context.ts

export interface BaseContext {
  // Request information - readonly
  readonly request: {
    readonly headers: Headers;
    readonly cookies: ReadonlyMap<string, string>;
    readonly ip: string;
  };

  // Response helpers - methods can modify state
  readonly response: {
    setHeader(name: string, value: string): void;
    setCookie(name: string, value: string, options?: CookieOptions): void;
    deleteCookie(name: string): void;
  };

  // Authentication (populated by @requireAuth)
  readonly auth: {
    readonly user: AuthenticatedUser | null;
    readonly isAuthenticated: boolean;
    hasRole(role: string): boolean;
    hasScope(scope: string): boolean;
  };

  // DataLoaders
  readonly loaders: DataLoaders;

  // AbortSignal for cancellation
  readonly signal: AbortSignal;
}

export interface DataLoaders {
  readonly user: DataLoader<UserId, User | null>;
  readonly post: DataLoader<PostId, Post | null>;
  readonly userPosts: DataLoader<UserId, ReadonlyArray<Post>>;
}
```

## 6. Server Implementation

```typescript
// src/server.ts
import { createServer } from "@better-graphql/server";
import type { Resolvers, BaseContext } from "./generated";
import { createDataLoaders } from "./loaders";

// Define resolvers
const resolvers: Resolvers<AppContext> = {
  Query: {
    me: async (_, __, ctx) => {
      return ctx.auth.user;
    },

    user: async (_, { id }, ctx) => {
      const user = await ctx.loaders.user.load(id);
      if (!user) {
        return {
          __typename: "NotFoundError",
          message: "User not found",
          code: "NOT_FOUND",
          resourceType: "User",
          resourceId: id,
        } satisfies NotFoundError;
      }
      return user;
    },

    users: async (_, { first = 10, after, filter, orderBy }, ctx) => {
      return ctx.db.users.paginate({ first, after, filter, orderBy });
    },
  },

  Mutation: {
    createUser: async (_, { input }, ctx) => {
      // Validation is automatic from @email, @minLength, etc.
      // If validation fails, ValidationError is returned automatically

      const existing = await ctx.db.users.findByEmail(input.email);
      if (existing) {
        return {
          __typename: "EmailAlreadyExistsError",
          message: "Email already registered",
          code: "EMAIL_EXISTS",
          existingEmail: input.email,
        } satisfies EmailAlreadyExistsError;
      }

      const user = await ctx.db.users.create(input);
      return user;
    },
  },

  User: {
    posts: async (parent, { first, after }, ctx) => {
      return ctx.db.posts.findByAuthor(parent.id, { first, after });
    },

    postsCount: async (parent, _, ctx) => {
      return ctx.loaders.userPostsCount.load(parent.id);
    },
  },

  Subscription: {
    postCreated: {
      subscribe: async function* (_, { authorId }, ctx) {
        const subscription = ctx.pubsub.subscribe("POST_CREATED", { authorId });

        for await (const post of subscription) {
          yield post;
        }
      },
    },
  },
};

// Create server
const server = createServer({
  schema: "./schema.bgql",
  resolvers,
  context: async (req): Promise<AppContext> => {
    const token = req.headers.get("Authorization")?.replace("Bearer ", "");
    const user = token ? await verifyToken(token) : null;

    return {
      request: req,
      response: req.response,
      auth: {
        user,
        isAuthenticated: !!user,
        hasRole: (role) => user?.roles.includes(role) ?? false,
        hasScope: (scope) => user?.scopes.includes(scope) ?? false,
      },
      loaders: createDataLoaders(),
      signal: req.signal,
      db: createDbClient(),
      pubsub: createPubSub(),
    };
  },
});

// Start server
server.listen({ port: 4000 });
```

## 7. DataLoader Implementation

> **Note**: The examples below use Prisma, but you can use any ORM or query builder you prefer (Drizzle, Kysely, TypeORM, raw SQL, etc.).

```typescript
// src/loaders.ts
import type { LoaderImplementations } from "./generated/loaders";
import type { PrismaClient } from "@prisma/client";

export function createLoaders(prisma: PrismaClient): LoaderImplementations {
  return {
    // Called once with all userIds collected in the batch window
    userPosts: async (userIds, args) => {
      const posts = await prisma.post.findMany({
        where: {
          authorId: { in: userIds.map(id => id.value) },
        },
        orderBy: { createdAt: "desc" },
        take: args.first ?? 10,
      });

      // Group by authorId
      const grouped = new Map<string, Post[]>();
      for (const post of posts) {
        const list = grouped.get(post.authorId) ?? [];
        list.push(post);
        grouped.set(post.authorId, list);
      }

      return new Map(userIds.map(id => [id, grouped.get(id.value) ?? []]));
    },

    userPostsCount: async (userIds) => {
      const counts = await prisma.post.groupBy({
        by: ["authorId"],
        where: {
          authorId: { in: userIds.map(id => id.value) },
        },
        _count: { id: true },
      });

      const countMap = new Map(counts.map(r => [r.authorId, r._count.id]));
      return new Map(userIds.map(id => [id, countMap.get(id.value) ?? 0]));
    },
  };
}

// Loaders are automatically injected and managed by the SDK
// No need to manually create DataLoader instances

// In resolvers, field batching happens automatically
const resolvers: Resolvers = {
  Query: {
    users: async (_, { first }, ctx) => {
      return ctx.prisma.user.findMany({ take: first });
    },
  },

  // Field resolvers are automatically batched
  User: {
    // SDK collects all User.posts requests and calls loader once
    posts: async (user, args, ctx) => {
      // This looks like it would cause N+1, but SDK batches automatically
      return ctx.prisma.post.findMany({
        where: { authorId: user.id.value },
        take: args.first ?? 10,
      });
    },
  },
};
```

## 8. Streaming Support

```typescript
// @defer support
const resolvers: Resolvers = {
  Query: {
    user: async (_, { id }, ctx) => {
      const user = await ctx.loaders.user.load(id);
      if (!user) return notFoundError("User", id);

      return {
        ...user,
        // Deferred fields return Promise
        // SDK automatically handles @defer streaming
      };
    },
  },

  User: {
    // This resolver is called lazily for @defer
    posts: async (parent, args, ctx) => {
      // Respects AbortSignal
      if (ctx.signal.aborted) {
        throw new AbortError("Request aborted");
      }
      return ctx.db.posts.findByAuthor(parent.id, args);
    },

    // Heavy computation deferred
    recommendations: async (parent, _, ctx) => {
      return ctx.ml.getRecommendations(parent.id);
    },
  },
};

// @stream support
const streamingResolvers: Resolvers = {
  Query: {
    posts: async function* (_, { first }, ctx) {
      // Generator for streaming
      const cursor = ctx.db.posts.cursor({ first });

      for await (const batch of cursor) {
        for (const post of batch) {
          if (ctx.signal.aborted) return;
          yield post;
        }
      }
    },
  },
};
```

## 9. File Upload Handling

```typescript
// File upload resolver
const resolvers: Resolvers = {
  Mutation: {
    uploadAvatar: async (_, { file }, ctx) => {
      // file is a Stream
      const buffer = await streamToBuffer(file.content);

      // Validate file
      if (buffer.length > 5 * 1024 * 1024) {
        return validationError("file", "File too large (max 5MB)");
      }

      // Upload to storage
      const url = await ctx.storage.upload({
        key: `avatars/${ctx.auth.user!.id}`,
        content: buffer,
        contentType: file.mimeType,
      });

      return {
        name: file.name,
        mimeType: file.mimeType,
        size: buffer.length,
        url,
      };
    },

    uploadVideo: async (_, { video }, ctx) => {
      // Stream directly to storage (no buffering)
      const { url, id } = await ctx.storage.streamUpload({
        content: video.content,
        contentType: `video/${video.format.toLowerCase()}`,
        onProgress: (loaded, total) => {
          ctx.pubsub.publish("UPLOAD_PROGRESS", { id, loaded, total });
        },
      });

      // Trigger transcoding job
      if (video.generateHls) {
        await ctx.transcoder.queueJob({
          videoId: id,
          variants: video.targetQualities ?? ["720p", "1080p"],
        });
      }

      return {
        id,
        url,
        format: video.format,
        hlsUrl: null,
        variants: [],
      };
    },
  },
};
```

## 10. Advanced Type Inference

### 10.1 Schema-to-Type Inference

The SDK provides zero-configuration type inference from schema to resolver implementation:

```typescript
// Schema: schema.bgql
// type Query {
//   user(id: UserId): UserResult
//   users(first: Int = 10, filter: Option<UserFilter>): UserConnection
// }
//
// union UserResult = User | NotFoundError | UnauthorizedError

// Generated with EXACT type inference
export interface QueryResolvers<TContext> {
  user: (
    parent: {},
    args: { readonly id: UserId },  // Required - no default
    context: TContext,
    info: ResolveInfo
  ) => Promise<UserResult> | UserResult;

  users: (
    parent: {},
    args: {
      readonly first: number;             // Has default, so not optional at runtime
      readonly filter: UserFilter | null; // Option<T> becomes T | null
    },
    context: TContext,
    info: ResolveInfo
  ) => Promise<UserConnection> | UserConnection;
}
```

### 10.2 Context Type Inference

Context types flow through the entire resolver chain:

```typescript
// Define your app context
interface AppContext extends BaseContext {
  readonly db: Database;
  readonly auth: {
    readonly user: User | null;
    hasPermission: (perm: Permission) => boolean;
  };
  readonly cache: Cache;
  readonly pubsub: PubSub;
}

// Resolvers automatically use your context type
const resolvers: Resolvers<AppContext> = {
  Query: {
    me: async (_, __, ctx) => {
      //                 ^? ctx: AppContext (inferred)
      return ctx.auth.user;
      //     ^? ctx.auth.user: User | null (inferred)
    },
  },
};
```

### 10.3 Discriminated Union Inference

Return types are strictly checked:

```typescript
// Schema: union CreateUserResult = User | ValidationError | EmailExistsError

const resolvers: Resolvers<AppContext> = {
  Mutation: {
    createUser: async (_, { input }, ctx) => {
      // TypeScript enforces correct return types

      // Valid: return ValidationError
      return {
        __typename: "ValidationError",
        message: "Invalid email",
        field: "email",
        constraint: "@email",
        code: "VALIDATION",
      } satisfies ValidationError;

      // Error: InvalidType not in union
      return {
        __typename: "InvalidType",  // ✗ Error: not assignable
        message: "Wrong",
      };
    },
  },
};
```

### 10.4 Type-Safe Utility Functions

```typescript
// Type guard for authenticated context
function isAuthenticated(ctx: Context): ctx is AuthenticatedContext {
  return ctx.auth.user !== null && ctx.auth.user !== undefined;
}

// Generic resolver wrapper with inference - no type assertions
function withAuth<TArgs, TResult>(
  resolver: (args: TArgs, ctx: AuthenticatedContext) => Promise<TResult>
): (parent: {}, args: TArgs, ctx: Context) => Promise<TResult | UnauthorizedError> {
  return async (parent, args, ctx) => {
    if (!isAuthenticated(ctx)) {
      return {
        __typename: "UnauthorizedError",
        message: "Authentication required",
        code: "UNAUTHORIZED",
      } satisfies UnauthorizedError;
    }
    // ctx is now narrowed to AuthenticatedContext by the type guard
    return resolver(args, ctx);
  };
}

// Usage with full inference
const resolvers = {
  Query: {
    me: withAuth(async (args, ctx) => {
      //                      ^? ctx: AuthenticatedContext (narrowed)
      return ctx.auth.user;
      //     ^? User (guaranteed non-null)
    }),
  },
};
```

## 11. Performance Optimizations

### 11.1 Query Planning

```typescript
const server = createServer({
  optimizations: {
    // Batch field resolution
    batchResolvers: true,

    // Parallel execution where possible
    parallelExecution: true,

    // Skip unused fields
    fieldPruning: true,

    // Cache parsed queries
    queryCache: true,

    // Prepared statements for DataLoaders
    preparedStatements: true,
  },
});
```

### 11.2 Response Caching

```typescript
const server = createServer({
  cache: {
    // In-memory cache for @cache directive
    store: new MemoryCache({ maxSize: "100mb" }),

    // Redis for distributed caching
    distributed: new RedisCache({ url: process.env.REDIS_URL }),

    // Cache key generation
    keyGenerator: (info, args) => {
      return `${info.parentType}:${info.fieldName}:${hash(args)}`;
    },
  },
});
```

## 12. Observability

### 12.1 Tracing

```typescript
const server = createServer({
  tracing: {
    // OpenTelemetry integration
    provider: new OTelTracerProvider(),

    // Trace all resolvers
    traceResolvers: true,

    // Include variables in spans (careful with PII)
    includeVariables: process.env.NODE_ENV === "development",
  },
});
```

### 12.2 Metrics

```typescript
const server = createServer({
  metrics: {
    // Prometheus metrics
    prometheus: {
      port: 9090,
      path: "/metrics",
    },

    // Collected metrics
    collect: [
      "query_duration_seconds",
      "resolver_duration_seconds",
      "dataloader_batch_size",
      "cache_hit_ratio",
      "active_subscriptions",
    ],
  },
});
```

### 12.3 Logging

```typescript
const server = createServer({
  logging: {
    // Structured logging
    format: "json",

    // Log levels per operation type
    levels: {
      query: "info",
      mutation: "info",
      subscription: "debug",
      error: "error",
    },

    // Redact sensitive fields
    redact: ["password", "token", "secret"],
  },
});
```

## 13. Security

### 13.1 Query Complexity Analysis

```typescript
const server = createServer({
  security: {
    // Limit query complexity
    maxComplexity: 1000,

    // Limit query depth
    maxDepth: 10,

    // Rate limiting per IP/user
    rateLimit: {
      window: "1m",
      max: 100,
    },
  },
});
```

### 13.2 Input Validation

```typescript
// Automatic validation from schema directives
// @email, @minLength, @maxLength, @pattern, etc.

// Custom validation
const server = createServer({
  validation: {
    custom: {
      password: (value) => {
        if (value.length < 8) {
          return "Password must be at least 8 characters";
        }
        if (!/[A-Z]/.test(value)) {
          return "Password must contain uppercase letter";
        }
        return null; // Valid
      },
    },
  },
});
```

## 14. Testing

```typescript
import { createTestClient } from "@better-graphql/testing";
import { resolvers } from "./resolvers";

describe("User queries", () => {
  const client = createTestClient({
    schema: "./schema.bgql",
    resolvers,
    context: {
      auth: { user: mockUser, isAuthenticated: true },
      loaders: createMockLoaders(),
    },
  });

  it("returns user by id", async () => {
    const result = await client.query({
      query: `
        query GetUser($id: UserId) {
          user(id: $id) {
            ... on User {
              id
              name
            }
            ... on NotFoundError {
              message
            }
          }
        }
      `,
      variables: { id: "user_1" },
    });

    expect(result.data.user.__typename).toBe("User");
    expect(result.data.user.name).toBe("John");
  });
});
```
