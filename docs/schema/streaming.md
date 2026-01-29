# Streaming

Better GraphQL supports incremental delivery with `@defer` and `@stream` directives for improved perceived performance.

## Overview

Streaming allows responses to be delivered incrementally:

- **@defer** - Delay expensive fields, send them later
- **@stream** - Stream list items one at a time

This improves Time to First Byte (TTFB) and perceived performance.

## @defer Directive

### Basic Usage

```graphql
query GetUser($id: ID!) {
  user(id: $id) {
    id
    name

    # Expensive field - defer it
    ... @defer {
      analytics {
        totalPosts
        totalLikes
        engagementRate
      }
    }
  }
}
```

### How It Works

1. Server sends initial response immediately:
```json
{
  "data": {
    "user": {
      "id": "1",
      "name": "John Doe"
    }
  },
  "hasNext": true
}
```

2. Deferred data arrives later:
```json
{
  "incremental": [{
    "path": ["user"],
    "data": {
      "analytics": {
        "totalPosts": 42,
        "totalLikes": 1337,
        "engagementRate": 4.2
      }
    }
  }],
  "hasNext": false
}
```

### Labeled Defer

```graphql
query GetUser($id: ID!) {
  user(id: $id) {
    id
    name

    ... @defer(label: "analytics") {
      analytics { ... }
    }

    ... @defer(label: "recommendations") {
      recommendations { ... }
    }
  }
}
```

### Conditional Defer

```graphql
query GetUser($id: ID!, $includeAnalytics: Boolean!) {
  user(id: $id) {
    id
    name

    ... @defer(if: $includeAnalytics) {
      analytics { ... }
    }
  }
}
```

## @stream Directive

### Basic Usage

```graphql
query GetPosts {
  posts(first: 100) @stream(initialCount: 10) {
    id
    title
    content
  }
}
```

### How It Works

1. Initial response with first 10 items:
```json
{
  "data": {
    "posts": [
      { "id": "1", "title": "First", "content": "..." },
      // ... items 2-10
    ]
  },
  "hasNext": true
}
```

2. Remaining items streamed incrementally:
```json
{
  "incremental": [{
    "path": ["posts", 10],
    "items": [
      { "id": "11", "title": "Eleventh", "content": "..." }
    ]
  }],
  "hasNext": true
}
```

### Labeled Stream

```graphql
query GetFeed {
  feed @stream(initialCount: 5, label: "feed-items") {
    ... on Post { id title }
    ... on Comment { id content }
  }
}
```

## Client Integration

### Using with @bgql/client

```typescript
import { createClient } from '@bgql/client';

const client = createClient('http://localhost:4000/graphql');

// Handle streaming response
const stream = client.queryStream(GetUserDocument, { id: '1' });

for await (const result of stream) {
  if (result.ok) {
    // Update UI with partial data
    updateUI(result.value);

    if (!result.hasNext) {
      // All data received
      break;
    }
  }
}
```

### Vue Integration

```vue
<script setup lang="ts">
import { useStreamingQuery } from '@bgql/client/vue';
import { GetUserDocument } from './generated/graphql';

const { data, loading, hasNext } = useStreamingQuery(GetUserDocument, {
  variables: { id: '1' },
});
</script>

<template>
  <div v-if="loading && !data">Loading...</div>
  <div v-else>
    <h1>{{ data.user.name }}</h1>

    <!-- Show skeleton while analytics loads -->
    <div v-if="hasNext" class="skeleton">Loading analytics...</div>
    <Analytics v-else :data="data.user.analytics" />
  </div>
</template>
```

### React Integration

```tsx
import { useStreamingQuery } from '@bgql/client/react';
import { GetUserDocument } from './generated/graphql';

function UserProfile({ id }: { id: string }) {
  const { data, loading, hasNext } = useStreamingQuery(GetUserDocument, {
    variables: { id },
  });

  if (loading && !data) {
    return <Spinner />;
  }

  return (
    <div>
      <h1>{data.user.name}</h1>

      {hasNext ? (
        <AnalyticsSkeleton />
      ) : (
        <Analytics data={data.user.analytics} />
      )}
    </div>
  );
}
```

## Server Implementation

### Enabling Streaming

```typescript
import { serve } from '@bgql/server';

const server = await serve({
  schema: './schema.bgql',
  resolvers,
  streaming: {
    enabled: true,
    // Optional: configure chunk size
    chunkSize: 10,
  },
});
```

### Resolver for Deferred Fields

```typescript
const resolvers = {
  User: {
    // Expensive resolver - good candidate for @defer
    analytics: async (user, _, ctx) => {
      // This runs only when the deferred fragment is processed
      const stats = await ctx.analytics.getUserStats(user.id);
      return stats;
    },
  },
};
```

### Async Generator for Streaming

```typescript
const resolvers = {
  Query: {
    // Return async generator for @stream
    posts: async function* (_, { first }, ctx) {
      const cursor = ctx.db.posts.cursor({ limit: first });

      for await (const post of cursor) {
        yield post;
      }
    },
  },
};
```

## Transport

### HTTP Multipart

Default transport for streaming over HTTP:

```
Content-Type: multipart/mixed; boundary="-"

---
Content-Type: application/json

{"data":{"user":{"id":"1","name":"John"}},"hasNext":true}
---
Content-Type: application/json

{"incremental":[{"path":["user"],"data":{"analytics":{...}}}],"hasNext":false}
-----
```

### Server-Sent Events (SSE)

Alternative transport:

```typescript
const client = createClient('http://localhost:4000/graphql', {
  streamTransport: 'sse',
});
```

### WebSocket

For subscription-like streaming:

```typescript
const client = createClient('http://localhost:4000/graphql', {
  streamTransport: 'websocket',
});
```

## Best Practices

### 1. Defer Expensive Computations

```graphql
# Good: Defer analytics that require complex calculations
query GetDashboard {
  user {
    id
    name

    ... @defer {
      analytics {
        # These require aggregating millions of records
        lifetimeValue
        engagementScore
        churnProbability
      }
    }
  }
}
```

### 2. Stream Large Lists

```graphql
# Good: Stream large result sets
query GetFeed {
  feed(first: 100) @stream(initialCount: 10) {
    id
    content
    author { name }
  }
}
```

### 3. Provide Meaningful Initial Data

```graphql
# Good: Initial response is useful
query GetUser($id: ID!) {
  user(id: $id) {
    # Essential data first
    id
    name
    email
    avatarUrl

    # Nice-to-have data deferred
    ... @defer {
      recentActivity { ... }
      recommendations { ... }
    }
  }
}
```

### 4. Use Labels for Complex Queries

```graphql
# Good: Labels help identify which part loaded
query GetProfile($id: ID!) {
  user(id: $id) {
    id
    name

    ... @defer(label: "stats") {
      stats { followers following posts }
    }

    ... @defer(label: "activity") {
      recentActivity { ... }
    }

    ... @defer(label: "suggestions") {
      suggestedFollows { ... }
    }
  }
}
```

### 5. Handle Partial Data in UI

```vue
<template>
  <div>
    <!-- Always show what we have -->
    <h1>{{ user.name }}</h1>

    <!-- Graceful loading states for deferred data -->
    <StatsSection
      v-if="user.stats"
      :stats="user.stats"
    />
    <StatsSkeleton v-else />
  </div>
</template>
```

## Limitations

### Not Supported

- Streaming over HTTP GET (requires POST)
- Deferred mutations
- Streaming subscriptions (use regular subscriptions instead)

### Fallback Behavior

If client doesn't support streaming, server returns complete response:

```typescript
// Server detects client capabilities
const supportsStreaming = request.headers.accept?.includes('multipart/mixed');

if (!supportsStreaming) {
  // Return complete response
  return { data: await resolveAll(query) };
}
```

## Performance Tips

### Measure Before Optimizing

```typescript
// Add timing to identify slow fields
const resolvers = {
  User: {
    analytics: async (user, _, ctx) => {
      const start = Date.now();
      const result = await ctx.analytics.getUserStats(user.id);
      console.log(`analytics resolved in ${Date.now() - start}ms`);
      return result;
    },
  },
};
```

### Cache Deferred Results

```typescript
const resolvers = {
  User: {
    analytics: async (user, _, ctx) => {
      // Check cache first
      const cached = await ctx.cache.get(`analytics:${user.id}`);
      if (cached) return cached;

      // Compute and cache
      const stats = await computeAnalytics(user.id);
      await ctx.cache.set(`analytics:${user.id}`, stats, { ttl: 300 });
      return stats;
    },
  },
};
```

## Next Steps

- [Subscriptions](/backend/subscriptions)
- [Performance](/backend/performance)
- [Client Streaming](/frontend/streaming)
