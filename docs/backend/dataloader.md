# DataLoader

DataLoader solves the N+1 query problem by batching and caching database requests.

## The N+1 Problem

Consider this query:

```graphql
query {
  posts {           # 1 query for posts
    id
    title
    author {        # N queries for authors (one per post)
      name
    }
  }
}
```

Without DataLoader, if you have 100 posts, you'll make 101 database queries.

## Creating Loaders

### Basic Setup

```typescript
import { createLoaders } from '@bgql/server';
import DataLoader from 'dataloader';

function createLoaders(db: Database) {
  return {
    users: new DataLoader(async (ids: readonly string[]) => {
      const users = await db.users.findByIds([...ids]);
      // Return in same order as requested
      const userMap = new Map(users.map(u => [u.id, u]));
      return ids.map(id => userMap.get(id) ?? null);
    }),

    posts: new DataLoader(async (ids: readonly string[]) => {
      const posts = await db.posts.findByIds([...ids]);
      const postMap = new Map(posts.map(p => [p.id, p]));
      return ids.map(id => postMap.get(id) ?? null);
    }),
  };
}
```

### Per-Request Loaders

Create new loaders for each request:

```typescript
import { serve } from '@bgql/server';

serve({
  schema: './schema.bgql',
  resolvers,
  context: (req) => ({
    db: database,
    // New loaders for each request (important for caching!)
    loaders: createLoaders(database),
  }),
});
```

## Using Loaders in Resolvers

```typescript
const resolvers = defineResolvers({
  Query: {
    posts: async (_, __, { db }) => {
      return db.posts.findAll();
    },
  },

  Post: {
    // Uses DataLoader - batched automatically
    author: async (post, _, { loaders }) => {
      return loaders.users.load(post.authorId);
    },
  },

  User: {
    posts: async (user, _, { loaders }) => {
      return loaders.postsByUser.load(user.id);
    },
  },
});
```

## Advanced Loaders

### One-to-Many Relationships

```typescript
function createLoaders(db: Database) {
  return {
    // One-to-many: user -> posts
    postsByUser: new DataLoader(async (userIds: readonly string[]) => {
      const posts = await db.posts.findByUserIds([...userIds]);

      // Group posts by user
      const postsByUser = new Map<string, Post[]>();
      for (const post of posts) {
        const userPosts = postsByUser.get(post.authorId) || [];
        userPosts.push(post);
        postsByUser.set(post.authorId, userPosts);
      }

      return userIds.map(id => postsByUser.get(id) || []);
    }),

    // Many-to-many: post -> tags
    tagsByPost: new DataLoader(async (postIds: readonly string[]) => {
      const relations = await db.postTags.findByPostIds([...postIds]);

      const tagIdsByPost = new Map<string, string[]>();
      for (const rel of relations) {
        const ids = tagIdsByPost.get(rel.postId) || [];
        ids.push(rel.tagId);
        tagIdsByPost.set(rel.postId, ids);
      }

      // Batch load all tags
      const allTagIds = [...new Set(relations.map(r => r.tagId))];
      const tags = await db.tags.findByIds(allTagIds);
      const tagMap = new Map(tags.map(t => [t.id, t]));

      return postIds.map(postId => {
        const tagIds = tagIdsByPost.get(postId) || [];
        return tagIds.map(id => tagMap.get(id)!);
      });
    }),
  };
}
```

### With Options

```typescript
const userLoader = new DataLoader(
  async (ids: readonly string[]) => {
    // Batch function
    const users = await db.users.findByIds([...ids]);
    const map = new Map(users.map(u => [u.id, u]));
    return ids.map(id => map.get(id) ?? null);
  },
  {
    // Enable caching (default: true)
    cache: true,

    // Custom cache key function
    cacheKeyFn: (key) => key.toString(),

    // Max batch size
    maxBatchSize: 100,

    // Batch scheduling function
    batchScheduleFn: (callback) => setTimeout(callback, 0),
  }
);
```

### Priming the Cache

```typescript
const resolvers = defineResolvers({
  Query: {
    user: async (_, { id }, { db, loaders }) => {
      const user = await db.users.findById(id);
      if (user) {
        // Prime the cache for future requests
        loaders.users.prime(id, user);
      }
      return user;
    },

    users: async (_, __, { db, loaders }) => {
      const users = await db.users.findAll();
      // Prime cache for all users
      users.forEach(user => loaders.users.prime(user.id, user));
      return users;
    },
  },
});
```

### Clearing Cache

```typescript
const resolvers = defineResolvers({
  Mutation: {
    updateUser: async (_, { id, input }, { db, loaders }) => {
      const user = await db.users.update(id, input);

      // Clear stale cache entry
      loaders.users.clear(id);

      // Or clear all
      loaders.users.clearAll();

      return user;
    },
  },
});
```

## Connection Pattern with DataLoader

```typescript
function createLoaders(db: Database) {
  return {
    // Load connections with pagination
    userPosts: new DataLoader(
      async (keys: readonly { userId: string; first: number; after?: string }[]) => {
        // Group by unique parameters
        const results = await Promise.all(
          keys.map(({ userId, first, after }) =>
            db.posts.findByUser(userId, { first, after })
          )
        );
        return results;
      },
      {
        // Custom cache key for pagination params
        cacheKeyFn: ({ userId, first, after }) =>
          `${userId}:${first}:${after ?? ''}`,
      }
    ),
  };
}

// Usage in resolver
const resolvers = defineResolvers({
  User: {
    posts: async (user, { first, after }, { loaders }) => {
      return loaders.userPosts.load({
        userId: user.id,
        first: first ?? 10,
        after,
      });
    },
  },
});
```

## Error Handling

```typescript
const userLoader = new DataLoader(async (ids: readonly string[]) => {
  try {
    const users = await db.users.findByIds([...ids]);
    const map = new Map(users.map(u => [u.id, u]));

    return ids.map(id => {
      const user = map.get(id);
      if (!user) {
        // Return Error for missing items
        return new Error(`User ${id} not found`);
      }
      return user;
    });
  } catch (error) {
    // Return same error for all keys on total failure
    return ids.map(() => error as Error);
  }
});

// In resolver, errors are thrown when loading
const resolvers = defineResolvers({
  Post: {
    author: async (post, _, { loaders }) => {
      try {
        return await loaders.users.load(post.authorId);
      } catch (error) {
        // Handle or return error type
        return {
          __typename: 'NotFoundError',
          message: error.message,
        };
      }
    },
  },
});
```

## TypeScript Types

```typescript
import DataLoader from 'dataloader';

interface Loaders {
  users: DataLoader<string, User | null>;
  posts: DataLoader<string, Post | null>;
  postsByUser: DataLoader<string, Post[]>;
  commentsByPost: DataLoader<string, Comment[]>;
}

interface Context {
  db: Database;
  loaders: Loaders;
  user: User | null;
}

// In resolvers
const resolvers = defineResolvers<Context>({
  Post: {
    author: async (post, _, { loaders }) => {
      return loaders.users.load(post.authorId);
    },
  },
});
```

## Performance Monitoring

```typescript
function createLoaders(db: Database, metrics: Metrics) {
  return {
    users: new DataLoader(async (ids: readonly string[]) => {
      const start = Date.now();

      const users = await db.users.findByIds([...ids]);

      // Track batch metrics
      metrics.increment('dataloader.users.batches');
      metrics.histogram('dataloader.users.batch_size', ids.length);
      metrics.timing('dataloader.users.duration', Date.now() - start);

      const map = new Map(users.map(u => [u.id, u]));
      return ids.map(id => map.get(id) ?? null);
    }),
  };
}
```

## Best Practices

### 1. Create Per-Request Loaders

```typescript
// ✅ Good: New loaders per request
context: (req) => ({
  loaders: createLoaders(db),
})

// ❌ Bad: Shared loaders across requests
const sharedLoaders = createLoaders(db);
context: () => ({ loaders: sharedLoaders })
```

### 2. Return Results in Order

```typescript
// ✅ Good: Return in same order as input
async function batchUsers(ids: readonly string[]) {
  const users = await db.users.findByIds([...ids]);
  const map = new Map(users.map(u => [u.id, u]));
  return ids.map(id => map.get(id) ?? null);  // Same order!
}

// ❌ Bad: Return in different order
async function batchUsers(ids: readonly string[]) {
  return db.users.findByIds([...ids]);  // Order not guaranteed!
}
```

### 3. Use Appropriate Batch Sizes

```typescript
const loader = new DataLoader(batchFn, {
  maxBatchSize: 100,  // Prevent huge queries
});
```

### 4. Handle Missing Items

```typescript
// ✅ Good: Return null or Error for missing
return ids.map(id => map.get(id) ?? null);

// ❌ Bad: Skip missing (wrong array length)
return users;  // Might have fewer items than ids!
```

## Next Steps

- [Testing](/backend/testing)
- [Error Handling](/backend/errors)
- [Context](/backend/context)
