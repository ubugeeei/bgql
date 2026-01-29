# Backend Streaming

Server-side implementation of `@defer` and `@stream` for incremental delivery.

## Enabling Streaming

### Server Configuration

```typescript
import { serve } from '@bgql/server';

const server = await serve({
  schema: './schema.bgql',
  resolvers,
  streaming: {
    enabled: true,
    // Default chunk size for @stream
    defaultInitialCount: 10,
    // Maximum concurrent deferred fragments
    maxConcurrentDefers: 5,
  },
});
```

### Transport Configuration

```typescript
const server = await serve({
  // ...
  streaming: {
    enabled: true,
    transport: 'multipart',  // 'multipart' | 'sse' | 'websocket'
  },
});
```

## @defer Implementation

### How @defer Works

1. Query executor identifies deferred fragments
2. Initial response sent with non-deferred data
3. Deferred resolvers execute in background
4. Results streamed as they complete

### Resolver for Deferred Fields

```typescript
const resolvers = {
  User: {
    // This resolver is called when the deferred fragment is processed
    analytics: async (user, _, ctx) => {
      // Expensive operation
      const stats = await ctx.analytics.compute(user.id);
      return stats;
    },

    // Regular field - resolved immediately
    name: (user) => user.name,
  },
};
```

### Prioritizing Deferred Fragments

```typescript
const server = await serve({
  // ...
  streaming: {
    enabled: true,
    deferPriority: (fragment, ctx) => {
      // Higher priority = resolved first
      if (fragment.label === 'critical') return 10;
      if (fragment.label === 'analytics') return 1;
      return 5;
    },
  },
});
```

## @stream Implementation

### Async Generator Pattern

```typescript
const resolvers = {
  Query: {
    // Return async generator for streamable fields
    posts: async function* (_, { first }, ctx) {
      const cursor = ctx.db.posts.cursor({
        limit: first,
        batchSize: 10,
      });

      for await (const post of cursor) {
        yield post;
      }
    },

    // Alternative: Return array and let executor stream
    users: async (_, { first }, ctx) => {
      return ctx.db.users.findMany({ take: first });
    },
  },
};
```

### Streaming Database Cursors

```typescript
// Prisma example
async function* streamUsers(prisma: PrismaClient, options: StreamOptions) {
  let cursor: string | undefined;
  const batchSize = options.batchSize ?? 100;

  while (true) {
    const users = await prisma.user.findMany({
      take: batchSize,
      skip: cursor ? 1 : 0,
      cursor: cursor ? { id: cursor } : undefined,
      orderBy: { id: 'asc' },
    });

    if (users.length === 0) break;

    for (const user of users) {
      yield user;
    }

    cursor = users[users.length - 1].id;

    if (users.length < batchSize) break;
  }
}

const resolvers = {
  Query: {
    allUsers: async function* (_, __, ctx) {
      yield* streamUsers(ctx.prisma, { batchSize: 50 });
    },
  },
};
```

## Response Format

### Multipart Response

```http
HTTP/1.1 200 OK
Content-Type: multipart/mixed; boundary="---"

-----
Content-Type: application/json

{"data":{"user":{"id":"1","name":"John"}},"hasNext":true}
-----
Content-Type: application/json

{"incremental":[{"path":["user"],"data":{"analytics":{"views":1000}}}],"hasNext":false}
-------
```

### Server-Sent Events

```http
HTTP/1.1 200 OK
Content-Type: text/event-stream

event: next
data: {"data":{"user":{"id":"1","name":"John"}},"hasNext":true}

event: next
data: {"incremental":[{"path":["user"],"data":{"analytics":{"views":1000}}}],"hasNext":false}

event: complete
data: {}
```

## Error Handling

### Errors in Deferred Fragments

```typescript
const resolvers = {
  User: {
    analytics: async (user, _, ctx) => {
      try {
        return await ctx.analytics.compute(user.id);
      } catch (error) {
        // Error will be included in the incremental response
        throw new GraphQLError('Failed to load analytics', {
          extensions: { code: 'ANALYTICS_ERROR' },
        });
      }
    },
  },
};
```

### Response with Error

```json
{
  "incremental": [{
    "path": ["user"],
    "data": null,
    "errors": [{
      "message": "Failed to load analytics",
      "path": ["user", "analytics"],
      "extensions": { "code": "ANALYTICS_ERROR" }
    }]
  }],
  "hasNext": false
}
```

### Partial Success

```typescript
const server = await serve({
  // ...
  streaming: {
    enabled: true,
    // Continue streaming even if some fragments fail
    continueOnError: true,
  },
});
```

## Performance Optimization

### Parallel Defer Execution

```typescript
const server = await serve({
  // ...
  streaming: {
    enabled: true,
    // Execute multiple deferred fragments in parallel
    maxConcurrentDefers: 10,
  },
});
```

### Batching Stream Items

```typescript
const resolvers = {
  Query: {
    posts: async function* (_, { first }, ctx) {
      const batchSize = 10;
      let offset = 0;

      while (offset < first) {
        const batch = await ctx.db.posts.findMany({
          skip: offset,
          take: Math.min(batchSize, first - offset),
        });

        if (batch.length === 0) break;

        // Yield batch at once for efficiency
        for (const post of batch) {
          yield post;
        }

        offset += batch.length;
      }
    },
  },
};
```

### Caching Deferred Results

```typescript
const resolvers = {
  User: {
    analytics: async (user, _, ctx) => {
      const cacheKey = `analytics:${user.id}`;

      // Check cache first
      const cached = await ctx.cache.get(cacheKey);
      if (cached) return cached;

      // Compute and cache
      const analytics = await ctx.analytics.compute(user.id);
      await ctx.cache.set(cacheKey, analytics, { ttl: 300 });

      return analytics;
    },
  },
};
```

## Testing Streaming

### Test Client Support

```typescript
import { createTestClient } from '@bgql/server';

const client = createTestClient({
  schema: './schema.bgql',
  resolvers,
  streaming: { enabled: true },
});

describe('Streaming', () => {
  it('should defer analytics', async () => {
    const results = [];

    const stream = client.queryStream(`
      query {
        user(id: "1") {
          name
          ... @defer {
            analytics { views }
          }
        }
      }
    `);

    for await (const result of stream) {
      results.push(result);
    }

    // First result: immediate data
    expect(results[0].data.user.name).toBe('John');
    expect(results[0].hasNext).toBe(true);

    // Second result: deferred data
    expect(results[1].incremental[0].data.analytics.views).toBe(1000);
    expect(results[1].hasNext).toBe(false);
  });
});
```

### Mocking Slow Resolvers

```typescript
it('should handle slow deferred fields', async () => {
  const slowResolvers = {
    User: {
      analytics: async () => {
        await delay(1000);  // Simulate slow operation
        return { views: 1000 };
      },
    },
  };

  const client = createTestClient({
    schema: './schema.bgql',
    resolvers: { ...resolvers, ...slowResolvers },
  });

  const start = Date.now();
  const results = [];

  for await (const result of client.queryStream(query)) {
    results.push({ result, time: Date.now() - start });
  }

  // Initial response should be fast
  expect(results[0].time).toBeLessThan(100);

  // Deferred response after delay
  expect(results[1].time).toBeGreaterThan(900);
});
```

## Best Practices

### 1. Identify Expensive Fields

```typescript
// Good candidates for @defer:
// - Aggregations
// - External API calls
// - Complex computations

const resolvers = {
  User: {
    // Fast - don't defer
    name: (user) => user.name,

    // Slow - good for defer
    recommendations: async (user, _, ctx) => {
      return ctx.ml.getRecommendations(user.id);
    },
  },
};
```

### 2. Use Labels for Debugging

```graphql
query GetUser($id: ID!) {
  user(id: $id) {
    name

    ... @defer(label: "recommendations") {
      recommendations { ... }
    }

    ... @defer(label: "analytics") {
      analytics { ... }
    }
  }
}
```

### 3. Set Appropriate initialCount

```graphql
# Good: Show some content immediately
query GetPosts {
  posts(first: 100) @stream(initialCount: 10) {
    id
    title
  }
}

# Bad: No initial content
query GetPosts {
  posts(first: 100) @stream(initialCount: 0) {
    id
    title
  }
}
```

### 4. Handle Cancellation

```typescript
const resolvers = {
  Query: {
    posts: async function* (_, { first }, ctx) {
      const cursor = ctx.db.posts.cursor({ limit: first });

      try {
        for await (const post of cursor) {
          // Check if client disconnected
          if (ctx.signal?.aborted) {
            break;
          }
          yield post;
        }
      } finally {
        // Clean up resources
        await cursor.close();
      }
    },
  },
};
```

## Next Steps

- [Streaming Client](/frontend/streaming)
- [Subscriptions](/backend/subscriptions)
- [Performance](/backend/performance)
