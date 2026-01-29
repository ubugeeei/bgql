# Performance

Optimize your Better GraphQL server for maximum performance.

## Query Optimization

### DataLoader for N+1

```typescript
import DataLoader from 'dataloader';

// Create loaders per request
function createLoaders(db: Database) {
  return {
    users: new DataLoader(async (ids: readonly string[]) => {
      const users = await db.users.findMany({
        where: { id: { in: [...ids] } },
      });
      const map = new Map(users.map(u => [u.id, u]));
      return ids.map(id => map.get(id) ?? null);
    }),

    postsByAuthor: new DataLoader(async (authorIds: readonly string[]) => {
      const posts = await db.posts.findMany({
        where: { authorId: { in: [...authorIds] } },
      });
      const map = new Map<string, typeof posts>();
      for (const post of posts) {
        const existing = map.get(post.authorId) ?? [];
        existing.push(post);
        map.set(post.authorId, existing);
      }
      return authorIds.map(id => map.get(id) ?? []);
    }),
  };
}

// Use in context
const server = await serve({
  // ...
  context: async () => ({
    db,
    loaders: createLoaders(db),
  }),
});

// Use in resolvers
const resolvers = {
  Post: {
    author: (post, _, ctx) => ctx.loaders.users.load(post.authorId),
  },
  User: {
    posts: (user, _, ctx) => ctx.loaders.postsByAuthor.load(user.id),
  },
};
```

### Query Complexity Analysis

```typescript
const server = await serve({
  // ...
  complexity: {
    enabled: true,
    maxComplexity: 1000,

    // Custom field costs
    fieldCost: {
      'Query.search': 50,
      'Query.analytics': 100,
      'User.followers': 10,
    },

    // List multiplier
    listMultiplier: (args) => args.first ?? args.limit ?? 10,

    // Callback when complexity exceeded
    onExceeded: (complexity, max, ctx) => {
      ctx.logger.warn({ complexity, max }, 'Query complexity exceeded');
    },
  },
});
```

### Query Depth Limiting

```typescript
const server = await serve({
  // ...
  limits: {
    maxDepth: 10,
    maxAliases: 5,
    maxDirectives: 10,
  },
});
```

## Caching Strategies

### Normalized Cache

```typescript
import { createNormalizedCache } from '@bgql/server';

const cache = createNormalizedCache({
  // Time to live
  ttl: 300000,  // 5 minutes

  // Cache key extraction
  getId: (obj) => obj.id ?? obj._id,

  // Types to cache
  types: ['User', 'Post', 'Comment'],
});

const server = await serve({
  // ...
  cache,
});
```

### Redis Caching

```typescript
import Redis from 'ioredis';
import { createRedisCache } from '@bgql/server';

const redis = new Redis(process.env.REDIS_URL);

const cache = createRedisCache({
  client: redis,
  ttl: 300,
  prefix: 'bgql:',

  // Cache specific queries
  shouldCache: (operation) => {
    return operation.operationName !== 'IntrospectionQuery';
  },
});
```

### Field-Level Caching

```typescript
const resolvers = {
  Query: {
    // Cache expensive computations
    analytics: async (_, { period }, ctx) => {
      const cacheKey = `analytics:${period}`;

      const cached = await ctx.cache.get(cacheKey);
      if (cached) return cached;

      const result = await computeAnalytics(period);
      await ctx.cache.set(cacheKey, result, { ttl: 3600 });

      return result;
    },
  },
};
```

### Cache Directives

```graphql
type Query {
  # Cache for 1 hour
  popularPosts: List<Post> @cacheControl(maxAge: 3600)

  # Cache per user
  recommendations: List<Post> @cacheControl(maxAge: 300, scope: PRIVATE)

  # No caching
  currentUser: User @cacheControl(maxAge: 0)
}
```

## Parallel Execution

### Concurrent Resolvers

```typescript
const resolvers = {
  Query: {
    dashboard: async (_, __, ctx) => {
      // Execute in parallel
      const [user, posts, notifications] = await Promise.all([
        ctx.db.users.findById(ctx.userId),
        ctx.db.posts.findMany({ authorId: ctx.userId }),
        ctx.db.notifications.findMany({ userId: ctx.userId }),
      ]);

      return { user, posts, notifications };
    },
  },
};
```

### Automatic Parallelization

Better GraphQL automatically executes independent field resolvers in parallel:

```graphql
query Dashboard {
  user { name }       # These run in parallel
  posts { title }     # These run in parallel
  notifications { }   # These run in parallel
}
```

## Database Optimization

### Select Only Needed Fields

```typescript
import { getSelections } from '@bgql/server';

const resolvers = {
  Query: {
    users: async (_, args, ctx, info) => {
      // Get requested fields
      const fields = getSelections(info);

      // Select only needed columns
      return ctx.db.users.findMany({
        select: fields.reduce((acc, f) => ({ ...acc, [f]: true }), {}),
      });
    },
  },
};
```

### Eager Loading

```typescript
const resolvers = {
  Query: {
    posts: async (_, args, ctx, info) => {
      const selections = getSelections(info);

      // Include relations if requested
      const include: Record<string, boolean> = {};
      if (selections.includes('author')) include.author = true;
      if (selections.includes('comments')) include.comments = true;

      return ctx.db.posts.findMany({ include });
    },
  },
};
```

### Connection Pooling

```typescript
import { Pool } from 'pg';

const pool = new Pool({
  connectionString: process.env.DATABASE_URL,
  max: 20,
  idleTimeoutMillis: 30000,
  connectionTimeoutMillis: 2000,
});

const server = await serve({
  // ...
  context: async () => ({
    db: pool,
  }),
});
```

## Memory Optimization

### Streaming Large Results

```typescript
const resolvers = {
  Query: {
    // Stream instead of loading all at once
    exportUsers: async function* (_, __, ctx) {
      const cursor = ctx.db.users.cursor({ batchSize: 100 });

      for await (const user of cursor) {
        yield user;
      }
    },
  },
};
```

### Dispose Resources

```typescript
const server = await serve({
  // ...
  context: async ({ request }) => {
    const db = await createConnection();

    // Register cleanup
    request.signal.addEventListener('abort', () => {
      db.close();
    });

    return { db };
  },
});
```

## Response Optimization

### Compression

```typescript
const server = await serve({
  // ...
  compression: {
    enabled: true,
    threshold: 1024,  // Only compress > 1KB
    level: 6,         // Compression level (1-9)
  },
});
```

### Persisted Queries

```typescript
import { persistedQueries } from './generated/persisted-queries.json';

const server = await serve({
  // ...
  persistedQueries: {
    enabled: true,
    store: persistedQueries,
    // Only allow persisted queries in production
    allowArbitraryQueries: process.env.NODE_ENV !== 'production',
  },
});
```

### Automatic Persisted Queries (APQ)

```typescript
const server = await serve({
  // ...
  apq: {
    enabled: true,
    cache: redis,
    ttl: 86400,  // 24 hours
  },
});
```

## Profiling

### Query Tracing

```typescript
const server = await serve({
  // ...
  tracing: {
    enabled: process.env.NODE_ENV !== 'production',
    // Include resolver timings
    includeResolverTimings: true,
  },
});

// Response includes tracing extension
// {
//   "extensions": {
//     "tracing": {
//       "startTime": "...",
//       "endTime": "...",
//       "duration": 12345678,
//       "execution": {
//         "resolvers": [...]
//       }
//     }
//   }
// }
```

### Custom Profiling

```typescript
const resolvers = {
  Query: {
    complexQuery: async (_, args, ctx) => {
      const profiler = ctx.profiler.start('complexQuery');

      profiler.mark('database');
      const data = await ctx.db.query(...);

      profiler.mark('transform');
      const result = transform(data);

      profiler.mark('complete');
      profiler.end();

      return result;
    },
  },
};
```

## Benchmarking

### Load Testing

```typescript
// k6 script
import http from 'k6/http';
import { check } from 'k6';

export const options = {
  vus: 100,
  duration: '30s',
};

export default function () {
  const res = http.post(
    'http://localhost:4000/graphql',
    JSON.stringify({
      query: `query { users(first: 10) { id name } }`,
    }),
    { headers: { 'Content-Type': 'application/json' } }
  );

  check(res, {
    'status is 200': (r) => r.status === 200,
    'no errors': (r) => !r.json().errors,
    'response time < 200ms': (r) => r.timings.duration < 200,
  });
}
```

### Benchmarking Resolvers

```typescript
import { bench, describe } from 'vitest';

describe('resolver benchmarks', () => {
  bench('getUser', async () => {
    await client.query(GetUserDocument, { id: '1' });
  });

  bench('getUsers with pagination', async () => {
    await client.query(GetUsersDocument, { first: 100 });
  });
});
```

## Best Practices

### 1. Always Use DataLoader

```typescript
// Bad: N+1 queries
const resolvers = {
  Post: {
    author: (post, _, ctx) => ctx.db.users.findById(post.authorId),
  },
};

// Good: Batched queries
const resolvers = {
  Post: {
    author: (post, _, ctx) => ctx.loaders.users.load(post.authorId),
  },
};
```

### 2. Limit Response Size

```typescript
const server = await serve({
  limits: {
    maxComplexity: 1000,
    maxDepth: 10,
    maxListSize: 100,
  },
});
```

### 3. Cache Appropriately

```typescript
// Public data: cache aggressively
// User-specific: cache with user key
// Real-time: don't cache

const cacheConfig = {
  'Query.publicPosts': { ttl: 3600 },
  'Query.userFeed': { ttl: 60, scope: 'user' },
  'Query.liveData': { ttl: 0 },
};
```

### 4. Monitor and Alert

```typescript
const server = await serve({
  // ...
  monitoring: {
    slowQueryThreshold: 1000,  // ms
    onSlowQuery: (query, duration, ctx) => {
      ctx.logger.warn({ query, duration }, 'Slow query detected');
      ctx.metrics.increment('slow_queries');
    },
  },
});
```

## Next Steps

- [DataLoader](/backend/dataloader)
- [Production](/backend/production)
- [Caching](/frontend/caching)
