# Client Streaming

Handle `@defer` and `@stream` responses for incremental data loading.

## Overview

Streaming allows the server to send responses incrementally:
- **@defer** - Load expensive fields after initial response
- **@stream** - Stream list items one by one

## Basic Usage

### Streaming Query

```typescript
import { createClient } from '@bgql/client';

const client = createClient('http://localhost:4000/graphql');

// Use queryStream for incremental responses
const stream = client.queryStream(GetUserDocument, { id: '1' });

for await (const result of stream) {
  if (result.ok) {
    // Update UI with partial data
    updateUI(result.value);

    if (!result.hasNext) {
      // All data received
      console.log('Complete:', result.value);
    }
  }
}
```

### Query with @defer

```graphql
query GetUser($id: ID!) {
  user(id: $id) {
    id
    name

    ... @defer(label: "analytics") {
      analytics {
        views
        likes
        engagement
      }
    }
  }
}
```

```typescript
const stream = client.queryStream(GetUserDocument, { id: '1' });

for await (const result of stream) {
  if (result.ok) {
    console.log('Name:', result.value.user.name);  // Available immediately

    if (result.value.user.analytics) {
      console.log('Analytics:', result.value.user.analytics);  // Available later
    }
  }
}
```

## Vue Integration

### useStreamingQuery

```vue
<script setup lang="ts">
import { useStreamingQuery } from '@bgql/client/vue';
import { GetUserDocument } from './generated/graphql';

const { data, loading, hasNext, error } = useStreamingQuery(GetUserDocument, {
  variables: { id: '1' },
});
</script>

<template>
  <div v-if="loading && !data">Loading...</div>
  <div v-else-if="error">Error: {{ error.message }}</div>
  <div v-else>
    <h1>{{ data.user.name }}</h1>

    <!-- Show loading state for deferred fields -->
    <template v-if="data.user.analytics">
      <AnalyticsPanel :data="data.user.analytics" />
    </template>
    <template v-else-if="hasNext">
      <AnalyticsSkeleton />
    </template>
  </div>
</template>
```

### Tracking Deferred Fragments

```vue
<script setup lang="ts">
import { useStreamingQuery } from '@bgql/client/vue';

const { data, loading, fragments } = useStreamingQuery(GetUserDocument, {
  variables: { id: '1' },
});

// Track which fragments have loaded
const analyticsLoaded = computed(() => fragments.value.has('analytics'));
const recommendationsLoaded = computed(() => fragments.value.has('recommendations'));
</script>

<template>
  <div>
    <UserProfile :user="data?.user" />

    <section>
      <h2>Analytics</h2>
      <Analytics v-if="analyticsLoaded" :data="data.user.analytics" />
      <Skeleton v-else />
    </section>

    <section>
      <h2>Recommendations</h2>
      <Recommendations v-if="recommendationsLoaded" :items="data.user.recommendations" />
      <Skeleton v-else />
    </section>
  </div>
</template>
```

### Streaming Lists

```vue
<script setup lang="ts">
import { useStreamingQuery } from '@bgql/client/vue';

// Query with @stream
const { data, hasNext } = useStreamingQuery(GetPostsDocument, {
  variables: { first: 100 },
});

// Posts stream in incrementally
const posts = computed(() => data.value?.posts ?? []);
</script>

<template>
  <div>
    <PostCard v-for="post in posts" :key="post.id" :post="post" />

    <!-- Show loading indicator while streaming -->
    <div v-if="hasNext" class="loading">
      Loading more posts...
    </div>
  </div>
</template>
```

## React Integration

### useStreamingQuery Hook

```tsx
import { useStreamingQuery } from '@bgql/client/react';
import { GetUserDocument } from './generated/graphql';

function UserProfile({ id }: { id: string }) {
  const { data, loading, hasNext, error } = useStreamingQuery(GetUserDocument, {
    variables: { id },
  });

  if (loading && !data) {
    return <Spinner />;
  }

  if (error) {
    return <Error message={error.message} />;
  }

  return (
    <div>
      <h1>{data.user.name}</h1>

      {data.user.analytics ? (
        <Analytics data={data.user.analytics} />
      ) : hasNext ? (
        <AnalyticsSkeleton />
      ) : null}
    </div>
  );
}
```

### With Suspense

```tsx
import { Suspense } from 'react';
import { useStreamingQuery } from '@bgql/client/react';

function UserAnalytics({ id }: { id: string }) {
  const { data } = useStreamingQuery(GetUserDocument, {
    variables: { id },
    suspense: true,
  });

  // Will suspend until analytics available
  return <Analytics data={data.user.analytics} />;
}

function UserProfile({ id }: { id: string }) {
  return (
    <div>
      <UserHeader id={id} />
      <Suspense fallback={<AnalyticsSkeleton />}>
        <UserAnalytics id={id} />
      </Suspense>
    </div>
  );
}
```

## Progress Tracking

### Fragment Progress

```typescript
const { data, progress } = useStreamingQuery(GetDashboardDocument);

// progress = { total: 5, completed: 3, fragments: ['header', 'stats', 'chart'] }
```

```vue
<template>
  <div>
    <ProgressBar :value="progress.completed" :max="progress.total" />
    <Dashboard :data="data" :loadedFragments="progress.fragments" />
  </div>
</template>
```

### Custom Progress UI

```vue
<script setup lang="ts">
const { data, hasNext, fragments } = useStreamingQuery(GetDashboardDocument);

const sections = [
  { key: 'header', label: 'Header' },
  { key: 'stats', label: 'Statistics' },
  { key: 'chart', label: 'Charts' },
  { key: 'recent', label: 'Recent Activity' },
];
</script>

<template>
  <div class="progress">
    <div
      v-for="section in sections"
      :key="section.key"
      :class="['section', { loaded: fragments.has(section.key) }]"
    >
      <span class="icon">{{ fragments.has(section.key) ? '✓' : '○' }}</span>
      <span>{{ section.label }}</span>
    </div>
  </div>
</template>
```

## Error Handling

### Partial Errors

```typescript
const { data, errors, hasNext } = useStreamingQuery(GetUserDocument, {
  variables: { id: '1' },
});

// Some fragments may fail while others succeed
if (errors.value?.length) {
  for (const error of errors.value) {
    console.log(`Error at ${error.path}: ${error.message}`);
  }
}
```

```vue
<template>
  <div>
    <UserProfile :user="data?.user" />

    <!-- Show error for failed fragment -->
    <ErrorBanner
      v-if="errors?.some(e => e.path?.includes('analytics'))"
      message="Failed to load analytics"
      @retry="refetch"
    />
    <Analytics v-else-if="data?.user.analytics" :data="data.user.analytics" />
    <Skeleton v-else-if="hasNext" />
  </div>
</template>
```

### Network Errors

```typescript
const { data, error, hasNext, refetch } = useStreamingQuery(GetUserDocument, {
  variables: { id: '1' },
  onError: (error) => {
    if (error.type === 'network') {
      showNotification('Connection lost. Retrying...');
    }
  },
});
```

## Caching

### Cache Streaming Results

```typescript
const { data } = useStreamingQuery(GetUserDocument, {
  variables: { id: '1' },
  // Cache intermediate results
  cachePartialResults: true,
});
```

### Merge with Cache

```typescript
const { data } = useStreamingQuery(GetUserDocument, {
  variables: { id: '1' },
  // Start with cached data, stream updates
  initialData: () => cache.readQuery(GetUserDocument, { id: '1' }),
});
```

## Best Practices

### 1. Show Progressive Loading States

```vue
<template>
  <div>
    <!-- Always show available data -->
    <h1 v-if="data?.user">{{ data.user.name }}</h1>
    <Skeleton v-else />

    <!-- Skeleton for pending fragments -->
    <section v-for="section in sections" :key="section.key">
      <component
        :is="getComponent(section.key)"
        v-if="fragments.has(section.key)"
        :data="getSectionData(section.key)"
      />
      <Skeleton v-else-if="hasNext" :type="section.key" />
    </section>
  </div>
</template>
```

### 2. Prioritize Important Data

```graphql
query GetDashboard {
  # Critical data - no defer
  user {
    name
    avatar
  }

  # Important but can wait
  ... @defer(label: "stats") {
    stats { ... }
  }

  # Nice to have
  ... @defer(label: "recommendations") {
    recommendations { ... }
  }
}
```

### 3. Handle Cancellation

```vue
<script setup lang="ts">
import { onUnmounted } from 'vue';

const { stop } = useStreamingQuery(GetUserDocument, {
  variables: { id: '1' },
});

// Cancel stream on unmount
onUnmounted(() => {
  stop();
});

// Or cancel on navigation
watch(() => route.params.id, () => {
  stop();
});
</script>
```

### 4. Optimize Initial Content

```graphql
# Good: Show meaningful initial content
query GetUser($id: ID!) {
  user(id: $id) {
    # Essential - render immediately
    id
    name
    avatar

    # Can be deferred
    ... @defer {
      detailedProfile { ... }
    }
  }
}

# Bad: Empty initial state
query GetUser($id: ID!) {
  user(id: $id) {
    id
    ... @defer {
      name
      avatar
      profile { ... }
    }
  }
}
```

### 5. Stream Large Lists

```graphql
# Good: Stream with initial batch
query GetPosts {
  posts(first: 100) @stream(initialCount: 10) {
    id
    title
  }
}
```

```vue
<script setup lang="ts">
const { data, hasNext } = useStreamingQuery(GetPostsDocument);

// Render posts as they arrive
const posts = computed(() => data.value?.posts ?? []);
</script>

<template>
  <div>
    <PostCard v-for="post in posts" :key="post.id" :post="post" />
    <LoadingIndicator v-if="hasNext" />
  </div>
</template>
```

## Fallback Behavior

When streaming is not supported:

```typescript
const client = createClient('http://localhost:4000/graphql', {
  streaming: {
    // Fallback to regular query if streaming unavailable
    fallbackToQuery: true,
  },
});
```

## Next Steps

- [Backend Streaming](/backend/streaming)
- [Queries](/frontend/queries)
- [Caching](/frontend/caching)
