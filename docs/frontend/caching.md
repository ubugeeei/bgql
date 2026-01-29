# Client Caching

Better GraphQL provides flexible caching strategies for optimal performance.

## Caching Strategies

### Simple TTL Cache

```typescript
import { createClient, createTTLCache } from '@bgql/client';

const cache = createTTLCache({
  ttl: 60000,  // 60 seconds
  maxSize: 100,
});

const client = createClient('http://localhost:4000/graphql', {
  cache,
});
```

### Normalized Cache

```typescript
import { createClient, createNormalizedCache } from '@bgql/client';

const cache = createNormalizedCache({
  // Time to live
  ttl: 300000,  // 5 minutes

  // Extract ID from objects
  getId: (obj) => obj.id ?? obj._id,

  // Types to normalize
  types: {
    User: true,
    Post: true,
    Comment: true,
  },
});

const client = createClient('http://localhost:4000/graphql', {
  cache,
});
```

### Custom Cache Implementation

```typescript
import { Cache } from '@bgql/client';

class LocalStorageCache implements Cache {
  private prefix = 'bgql:';

  get(key: string): unknown | null {
    const item = localStorage.getItem(this.prefix + key);
    if (!item) return null;

    const { data, expiry } = JSON.parse(item);
    if (expiry && Date.now() > expiry) {
      localStorage.removeItem(this.prefix + key);
      return null;
    }

    return data;
  }

  set(key: string, value: unknown, options?: { ttl?: number }): void {
    const item = {
      data: value,
      expiry: options?.ttl ? Date.now() + options.ttl : null,
    };
    localStorage.setItem(this.prefix + key, JSON.stringify(item));
  }

  delete(key: string): void {
    localStorage.removeItem(this.prefix + key);
  }

  clear(): void {
    const keys = Object.keys(localStorage).filter(k => k.startsWith(this.prefix));
    keys.forEach(k => localStorage.removeItem(k));
  }
}

const client = createClient('http://localhost:4000/graphql', {
  cache: new LocalStorageCache(),
});
```

## Cache Policies

### Fetch Policies

```typescript
// Cache first (default) - return cached if available
const result = await client.query(GetUserDocument, { id: '1' }, {
  fetchPolicy: 'cache-first',
});

// Network only - always fetch from network
const result = await client.query(GetUserDocument, { id: '1' }, {
  fetchPolicy: 'network-only',
});

// Cache only - only return cached data
const result = await client.query(GetUserDocument, { id: '1' }, {
  fetchPolicy: 'cache-only',
});

// Network first - fetch from network, fall back to cache
const result = await client.query(GetUserDocument, { id: '1' }, {
  fetchPolicy: 'network-first',
});

// Cache and network - return cache immediately, then update with network
const result = await client.query(GetUserDocument, { id: '1' }, {
  fetchPolicy: 'cache-and-network',
});
```

### Vue Composable with Policies

```vue
<script setup lang="ts">
import { useQuery } from '@bgql/client/vue';
import { GetUserDocument } from './generated/graphql';

// Always fresh data
const { data, loading } = useQuery(GetUserDocument, {
  variables: { id: '1' },
  fetchPolicy: 'network-only',
});

// Cache first for fast initial load
const { data: cachedData } = useQuery(GetUsersDocument, {
  fetchPolicy: 'cache-first',
});
</script>
```

## Cache Updates

### Automatic Updates

Normalized cache automatically updates all references:

```typescript
// Query fetches user
const result = await client.query(GetUserDocument, { id: '1' });
// result.value.user.name === 'John'

// Mutation updates user
await client.mutate(UpdateUserDocument, {
  id: '1',
  input: { name: 'Jane' },
});

// Cache automatically updated
// Next query returns updated data
const updated = await client.query(GetUserDocument, { id: '1' });
// updated.value.user.name === 'Jane'
```

### Manual Cache Updates

```typescript
// Direct cache manipulation
client.cache.write('User:1', {
  __typename: 'User',
  id: '1',
  name: 'Updated Name',
});

// Update specific query
client.cache.writeQuery(GetUserDocument, { id: '1' }, {
  user: { __typename: 'User', id: '1', name: 'Updated' },
});

// Read from cache
const cached = client.cache.readQuery(GetUserDocument, { id: '1' });
```

### Mutation with Cache Update

```typescript
import { useMutation } from '@bgql/client/vue';

const { mutate } = useMutation(CreatePostDocument, {
  update: (cache, { data }) => {
    if (data?.createPost.__typename === 'Post') {
      // Read existing query
      const existing = cache.readQuery(GetPostsDocument);

      // Write updated query
      cache.writeQuery(GetPostsDocument, null, {
        posts: {
          ...existing.posts,
          edges: [
            { node: data.createPost, cursor: data.createPost.id },
            ...existing.posts.edges,
          ],
        },
      });
    }
  },
});
```

## Optimistic Updates

### Basic Optimistic Update

```typescript
const { mutate } = useMutation(UpdateUserDocument, {
  optimisticResponse: (variables) => ({
    updateUser: {
      __typename: 'User',
      id: variables.id,
      ...variables.input,
    },
  }),
});

// UI updates immediately, then syncs with server response
await mutate({
  id: '1',
  input: { name: 'New Name' },
});
```

### Optimistic with Rollback

```typescript
const { mutate } = useMutation(LikePostDocument, {
  optimisticResponse: (variables) => ({
    likePost: {
      __typename: 'Post',
      id: variables.postId,
      liked: true,
      likeCount: getCurrentLikeCount(variables.postId) + 1,
    },
  }),
  onError: (error, variables, context) => {
    // Automatically rolls back optimistic update
    console.error('Like failed:', error);
  },
});
```

## Cache Invalidation

### Invalidate by Type

```typescript
// Invalidate all User entries
client.cache.invalidate('User');

// Invalidate specific entity
client.cache.invalidate('User', '1');
```

### Invalidate by Query

```typescript
// Invalidate specific query
client.cache.invalidateQuery(GetUsersDocument);

// Invalidate with variables
client.cache.invalidateQuery(GetUserDocument, { id: '1' });
```

### Refetch Queries

```typescript
// After mutation, refetch related queries
const { mutate } = useMutation(CreatePostDocument, {
  refetchQueries: [
    GetPostsDocument,
    { document: GetUserDocument, variables: { id: currentUser.id } },
  ],
});
```

### Conditional Refetch

```typescript
const { mutate } = useMutation(UpdatePostDocument, {
  refetchQueries: (result) => {
    if (result.data?.updatePost.__typename === 'Post') {
      return [GetPostsDocument];
    }
    return [];
  },
});
```

## Garbage Collection

### Automatic GC

```typescript
const cache = createNormalizedCache({
  ttl: 300000,
  // Run GC every 5 minutes
  gcInterval: 300000,
  // Keep max 1000 entities
  maxSize: 1000,
});
```

### Manual GC

```typescript
// Remove expired entries
client.cache.gc();

// Clear entire cache
client.cache.clear();

// Remove specific types
client.cache.evict('TemporaryData');
```

## Persisting Cache

### Local Storage Persistence

```typescript
import { createNormalizedCache, persistCache } from '@bgql/client';

const cache = createNormalizedCache();

// Restore from storage
await persistCache.restore(cache, {
  storage: localStorage,
  key: 'bgql-cache',
});

// Persist on changes
persistCache.persist(cache, {
  storage: localStorage,
  key: 'bgql-cache',
  debounce: 1000,  // Debounce writes
});
```

### IndexedDB Persistence

```typescript
import { createIndexedDBStorage } from '@bgql/client';

const storage = await createIndexedDBStorage({
  dbName: 'bgql-cache',
  storeName: 'cache',
});

await persistCache.restore(cache, { storage });
persistCache.persist(cache, { storage });
```

## Framework Integration

### Vue Reactive Cache

```vue
<script setup lang="ts">
import { useQuery, useCache } from '@bgql/client/vue';

const cache = useCache();

// Reactive cache read
const user = computed(() => cache.read('User', '1'));

// Watch cache changes
watch(
  () => cache.read('User', '1'),
  (newUser) => {
    console.log('User updated:', newUser);
  }
);
</script>
```

### React Cache Hook

```tsx
import { useCache, useCacheSubscription } from '@bgql/client/react';

function UserStatus({ userId }: { userId: string }) {
  const cache = useCache();

  // Subscribe to cache updates
  const user = useCacheSubscription('User', userId);

  return <span>{user?.status}</span>;
}
```

## Best Practices

### 1. Use Normalized Cache for Complex Apps

```typescript
// Simple apps: TTL cache is sufficient
const cache = createTTLCache({ ttl: 60000 });

// Complex apps: normalized cache for consistency
const cache = createNormalizedCache({
  types: { User: true, Post: true },
});
```

### 2. Set Appropriate TTLs

```typescript
const cache = createNormalizedCache({
  ttl: 300000,  // Default: 5 minutes

  // Per-type TTLs
  typeTTL: {
    User: 600000,      // 10 minutes - changes rarely
    Notification: 60000, // 1 minute - changes often
    LiveData: 0,       // Never cache
  },
});
```

### 3. Invalidate on Mutations

```typescript
const { mutate } = useMutation(DeletePostDocument, {
  update: (cache, { data }) => {
    if (data?.deletePost) {
      // Remove from cache
      cache.evict('Post', data.deletePost.id);

      // Update list queries
      cache.invalidateQuery(GetPostsDocument);
    }
  },
});
```

### 4. Handle Stale Data

```typescript
const { data, loading, stale, refetch } = useQuery(GetUserDocument, {
  variables: { id: '1' },
  fetchPolicy: 'cache-and-network',
});

// Show indicator for stale data
<div :class="{ 'opacity-50': stale }">
  {{ data?.user.name }}
  <button v-if="stale" @click="refetch">Refresh</button>
</div>
```

## Next Steps

- [Queries](/frontend/queries)
- [Mutations](/frontend/mutations)
- [Performance](/backend/performance)
