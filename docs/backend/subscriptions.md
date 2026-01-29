# Subscriptions

Better GraphQL supports real-time subscriptions over WebSocket for live data updates.

## Basic Setup

### Schema Definition

```graphql
type Subscription {
  # Simple subscription
  messageCreated(channelId: ID): Message

  # Subscription with filters
  postUpdated(authorId: Option<ID>): Post

  # Subscription returning union
  userEvent(userId: ID): UserEvent
}

union UserEvent =
  | UserOnline
  | UserOffline
  | UserTyping

type UserOnline {
  user: User
  timestamp: DateTime
}

type UserOffline {
  user: User
  timestamp: DateTime
  lastSeen: DateTime
}

type UserTyping {
  user: User
  channelId: ID
}
```

### Server Configuration

```typescript
import { serve } from '@bgql/server';
import { createPubSub } from '@bgql/server/pubsub';

const pubsub = createPubSub();

const server = await serve({
  schema: './schema.bgql',
  resolvers,
  pubsub,
  subscriptions: {
    path: '/subscriptions',
    // Optional: Authentication for subscriptions
    onConnect: async (ctx, connectionParams) => {
      const token = connectionParams?.authToken;
      if (token) {
        const user = await verifyToken(token);
        return { user };
      }
      return {};
    },
    onDisconnect: async (ctx) => {
      if (ctx.user) {
        await setUserOffline(ctx.user.id);
      }
    },
  },
});
```

## Implementing Resolvers

### Basic Subscription

```typescript
const resolvers = {
  Subscription: {
    messageCreated: {
      subscribe: (_, { channelId }, ctx) => {
        // Return async iterator
        return ctx.pubsub.subscribe(`messages:${channelId}`);
      },
      resolve: (payload) => payload,
    },
  },
};
```

### With Filter

```typescript
const resolvers = {
  Subscription: {
    postUpdated: {
      subscribe: (_, { authorId }, ctx) => {
        return ctx.pubsub.subscribe('posts:updated', {
          filter: (payload) => {
            // Only send if no filter or matches author
            if (!authorId) return true;
            return payload.post.authorId === authorId;
          },
        });
      },
      resolve: (payload) => payload.post,
    },
  },
};
```

### Publishing Events

```typescript
const resolvers = {
  Mutation: {
    createMessage: async (_, { input }, ctx) => {
      const message = await ctx.db.messages.create(input);

      // Publish to subscribers
      ctx.pubsub.publish(`messages:${input.channelId}`, message);

      return { __typename: 'Message', ...message };
    },

    updatePost: async (_, { id, input }, ctx) => {
      const post = await ctx.db.posts.update(id, input);

      // Publish update event
      ctx.pubsub.publish('posts:updated', { post });

      return { __typename: 'Post', ...post };
    },
  },
};
```

## PubSub Implementations

### In-Memory PubSub

```typescript
import { createPubSub } from '@bgql/server/pubsub';

// Simple in-memory PubSub (single server only)
const pubsub = createPubSub();
```

### Redis PubSub

```typescript
import { createRedisPubSub } from '@bgql/server/pubsub';
import Redis from 'ioredis';

const redis = new Redis(process.env.REDIS_URL);

// Redis-backed PubSub (multi-server)
const pubsub = createRedisPubSub({
  publisher: redis,
  subscriber: redis.duplicate(),
});
```

### Custom PubSub

```typescript
import { PubSub } from '@bgql/server/pubsub';

class CustomPubSub implements PubSub {
  async publish(channel: string, payload: unknown): Promise<void> {
    // Custom publish logic
  }

  subscribe(channel: string, options?: SubscribeOptions): AsyncIterator<unknown> {
    // Return async iterator
  }
}
```

## Client Usage

### Basic Subscription

```typescript
import { createClient } from '@bgql/client';

const client = createClient('http://localhost:4000/graphql', {
  subscriptions: {
    url: 'ws://localhost:4000/subscriptions',
  },
});

// Subscribe to messages
const subscription = client.subscribe(MessageCreatedDocument, {
  channelId: 'channel-1',
});

subscription.subscribe({
  next: (result) => {
    if (result.ok) {
      console.log('New message:', result.value.messageCreated);
    }
  },
  error: (error) => {
    console.error('Subscription error:', error);
  },
  complete: () => {
    console.log('Subscription ended');
  },
});

// Later: unsubscribe
subscription.unsubscribe();
```

### Vue Composable

```vue
<script setup lang="ts">
import { useSubscription } from '@bgql/client/vue';
import { MessageCreatedDocument } from './generated/graphql';

const { data, error, loading } = useSubscription(MessageCreatedDocument, {
  variables: { channelId: 'channel-1' },
  onData: (message) => {
    // Handle new message
    messages.value.push(message);
  },
});
</script>

<template>
  <div v-if="loading">Connecting...</div>
  <div v-else-if="error">Error: {{ error.message }}</div>
  <div v-else>
    <MessageList :messages="messages" />
  </div>
</template>
```

### React Hook

```tsx
import { useSubscription } from '@bgql/client/react';
import { MessageCreatedDocument } from './generated/graphql';

function ChatRoom({ channelId }: { channelId: string }) {
  const [messages, setMessages] = useState<Message[]>([]);

  const { loading, error } = useSubscription(MessageCreatedDocument, {
    variables: { channelId },
    onData: (data) => {
      setMessages(prev => [...prev, data.messageCreated]);
    },
  });

  if (loading) return <div>Connecting...</div>;
  if (error) return <div>Error: {error.message}</div>;

  return <MessageList messages={messages} />;
}
```

## Authentication

### Connection Authentication

```typescript
// Client: Pass auth token on connect
const client = createClient('http://localhost:4000/graphql', {
  subscriptions: {
    url: 'ws://localhost:4000/subscriptions',
    connectionParams: () => ({
      authToken: localStorage.getItem('token'),
    }),
  },
});

// Server: Validate on connect
const server = await serve({
  // ...
  subscriptions: {
    onConnect: async (ctx, connectionParams) => {
      const token = connectionParams?.authToken;
      if (!token) {
        throw new Error('Authentication required');
      }

      const user = await verifyToken(token);
      return { user };
    },
  },
});
```

### Per-Subscription Authorization

```typescript
const resolvers = {
  Subscription: {
    privateMessages: {
      subscribe: async (_, { channelId }, ctx) => {
        // Check if user has access to channel
        const hasAccess = await ctx.db.channels.userHasAccess(
          channelId,
          ctx.user?.id
        );

        if (!hasAccess) {
          throw new Error('Access denied to this channel');
        }

        return ctx.pubsub.subscribe(`messages:${channelId}`);
      },
    },
  },
};
```

## Presence System

### Tracking Online Users

```typescript
// Schema
type Subscription {
  userPresence(roomId: ID): PresenceEvent
}

union PresenceEvent = UserJoined | UserLeft

type UserJoined {
  user: User
  timestamp: DateTime
}

type UserLeft {
  user: User
  timestamp: DateTime
}
```

```typescript
// Server
const onlineUsers = new Map<string, Set<string>>();

const server = await serve({
  // ...
  subscriptions: {
    onConnect: async (ctx, params) => {
      const user = await verifyToken(params.authToken);
      if (user) {
        const rooms = await getUserRooms(user.id);
        for (const roomId of rooms) {
          addUserToRoom(roomId, user.id);
          pubsub.publish(`presence:${roomId}`, {
            __typename: 'UserJoined',
            user,
            timestamp: new Date().toISOString(),
          });
        }
      }
      return { user };
    },

    onDisconnect: async (ctx) => {
      if (ctx.user) {
        const rooms = await getUserRooms(ctx.user.id);
        for (const roomId of rooms) {
          removeUserFromRoom(roomId, ctx.user.id);
          pubsub.publish(`presence:${roomId}`, {
            __typename: 'UserLeft',
            user: ctx.user,
            timestamp: new Date().toISOString(),
          });
        }
      }
    },
  },
});
```

## Typing Indicators

```graphql
type Subscription {
  typing(channelId: ID): TypingIndicator
}

type TypingIndicator {
  user: User
  isTyping: Boolean
}

type Mutation {
  setTyping(channelId: ID, isTyping: Boolean): Boolean
}
```

```typescript
const resolvers = {
  Mutation: {
    setTyping: async (_, { channelId, isTyping }, ctx) => {
      ctx.pubsub.publish(`typing:${channelId}`, {
        user: ctx.user,
        isTyping,
      });
      return true;
    },
  },

  Subscription: {
    typing: {
      subscribe: (_, { channelId }, ctx) => {
        return ctx.pubsub.subscribe(`typing:${channelId}`, {
          // Don't send typing events to the user who is typing
          filter: (payload) => payload.user.id !== ctx.user?.id,
        });
      },
    },
  },
};
```

## Best Practices

### 1. Use Namespaced Channels

```typescript
// Good: Namespaced channels
ctx.pubsub.subscribe(`messages:${channelId}`);
ctx.pubsub.subscribe(`users:${userId}:notifications`);
ctx.pubsub.subscribe(`posts:${postId}:comments`);

// Avoid: Generic channels
ctx.pubsub.subscribe('updates');
```

### 2. Clean Up Resources

```typescript
// Client: Unsubscribe when component unmounts
onUnmounted(() => {
  subscription.unsubscribe();
});

// Server: Handle disconnection
subscriptions: {
  onDisconnect: async (ctx) => {
    await cleanupUserResources(ctx.user?.id);
  },
}
```

### 3. Handle Reconnection

```typescript
const client = createClient('http://localhost:4000/graphql', {
  subscriptions: {
    url: 'ws://localhost:4000/subscriptions',
    reconnect: true,
    reconnectAttempts: 5,
    reconnectInterval: 3000,
    onReconnected: () => {
      console.log('Reconnected to subscriptions');
      // Re-fetch missed data
      refetchQueries();
    },
  },
});
```

### 4. Batch Updates

```typescript
// Debounce frequent updates
import { debounce } from 'lodash';

const debouncedPublish = debounce((channelId: string, payload: unknown) => {
  pubsub.publish(`updates:${channelId}`, payload);
}, 100);

// Or batch multiple updates
const pendingUpdates = new Map<string, unknown[]>();

function queueUpdate(channelId: string, update: unknown) {
  const updates = pendingUpdates.get(channelId) ?? [];
  updates.push(update);
  pendingUpdates.set(channelId, updates);
}

setInterval(() => {
  for (const [channelId, updates] of pendingUpdates) {
    if (updates.length > 0) {
      pubsub.publish(`updates:${channelId}`, { updates });
      pendingUpdates.set(channelId, []);
    }
  }
}, 100);
```

### 5. Scale with Redis

```typescript
// For multi-server deployments, use Redis PubSub
import { createRedisPubSub } from '@bgql/server/pubsub';

const pubsub = createRedisPubSub({
  // Separate connections for pub and sub
  publisher: new Redis(process.env.REDIS_URL),
  subscriber: new Redis(process.env.REDIS_URL),
});
```

## Error Handling

### Subscription Errors

```typescript
// Client-side error handling
subscription.subscribe({
  next: (result) => {
    if (result.ok) {
      handleData(result.value);
    } else {
      handleError(result.error);
    }
  },
  error: (error) => {
    if (error.code === 'UNAUTHORIZED') {
      redirectToLogin();
    } else {
      showErrorNotification(error.message);
    }
  },
});
```

### Graceful Degradation

```typescript
// Fall back to polling if WebSocket unavailable
const client = createClient('http://localhost:4000/graphql', {
  subscriptions: {
    url: 'ws://localhost:4000/subscriptions',
    fallback: 'polling',
    pollingInterval: 5000,
  },
});
```

## Next Steps

- [Authentication](/backend/authentication)
- [Streaming](/backend/streaming)
- [Performance](/backend/performance)
