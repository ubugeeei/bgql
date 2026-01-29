# Client Subscriptions

Real-time updates with Better GraphQL subscriptions.

## Setup

### Client Configuration

```typescript
import { createClient } from '@bgql/client';

const client = createClient('http://localhost:4000/graphql', {
  subscriptions: {
    url: 'ws://localhost:4000/subscriptions',
    // Reconnection settings
    reconnect: true,
    reconnectAttempts: 5,
    reconnectInterval: 3000,
  },
});
```

### With Authentication

```typescript
const client = createClient('http://localhost:4000/graphql', {
  subscriptions: {
    url: 'ws://localhost:4000/subscriptions',
    connectionParams: () => ({
      authToken: localStorage.getItem('token'),
    }),
    // Reconnect with fresh token
    lazy: true,
  },
});
```

## Basic Usage

### Subscribing to Events

```typescript
const subscription = client.subscribe(MessageCreatedDocument, {
  channelId: 'channel-1',
});

const unsubscribe = subscription.subscribe({
  next: (result) => {
    if (result.ok) {
      console.log('New message:', result.value.messageCreated);
    }
  },
  error: (error) => {
    console.error('Subscription error:', error);
  },
  complete: () => {
    console.log('Subscription completed');
  },
});

// Later: cleanup
unsubscribe();
```

### Async Iterator

```typescript
const subscription = client.subscribe(MessageCreatedDocument, {
  channelId: 'channel-1',
});

for await (const result of subscription) {
  if (result.ok) {
    handleMessage(result.value.messageCreated);
  }
}
```

## Vue Integration

### useSubscription Composable

```vue
<script setup lang="ts">
import { ref } from 'vue';
import { useSubscription } from '@bgql/client/vue';
import { MessageCreatedDocument } from './generated/graphql';

const messages = ref<Message[]>([]);

const { loading, error } = useSubscription(MessageCreatedDocument, {
  variables: { channelId: 'channel-1' },
  onData: (data) => {
    messages.value.push(data.messageCreated);
  },
});
</script>

<template>
  <div v-if="loading">Connecting...</div>
  <div v-else-if="error">Error: {{ error.message }}</div>
  <div v-else>
    <div v-for="message in messages" :key="message.id">
      {{ message.content }}
    </div>
  </div>
</template>
```

### Conditional Subscription

```vue
<script setup lang="ts">
import { useSubscription } from '@bgql/client/vue';

const props = defineProps<{
  channelId: string;
  enabled: boolean;
}>();

const { data, loading } = useSubscription(MessageCreatedDocument, {
  variables: () => ({ channelId: props.channelId }),
  enabled: () => props.enabled,
});
</script>
```

### With Initial Query

```vue
<script setup lang="ts">
import { useQuery, useSubscription } from '@bgql/client/vue';

const { data: initialData, loading: queryLoading } = useQuery(GetMessagesDocument, {
  variables: { channelId: 'channel-1', last: 50 },
});

const messages = ref<Message[]>([]);

// Populate with initial data
watch(initialData, (data) => {
  if (data?.messages) {
    messages.value = [...data.messages.edges.map(e => e.node)];
  }
});

// Subscribe to new messages
useSubscription(MessageCreatedDocument, {
  variables: { channelId: 'channel-1' },
  enabled: () => !queryLoading.value,
  onData: (data) => {
    messages.value.push(data.messageCreated);
  },
});
</script>
```

## React Integration

### useSubscription Hook

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

  return (
    <div>
      {messages.map(msg => (
        <Message key={msg.id} message={msg} />
      ))}
    </div>
  );
}
```

### With Suspense

```tsx
import { Suspense } from 'react';
import { useSubscription } from '@bgql/client/react';

function ChatMessages({ channelId }: { channelId: string }) {
  const { data } = useSubscription(MessageCreatedDocument, {
    variables: { channelId },
    suspense: true,
  });

  return <Message message={data.messageCreated} />;
}

function ChatRoom({ channelId }: { channelId: string }) {
  return (
    <Suspense fallback={<div>Connecting...</div>}>
      <ChatMessages channelId={channelId} />
    </Suspense>
  );
}
```

## Handling Events

### Union Type Events

```graphql
subscription OnUserEvent($userId: ID!) {
  userEvent(userId: $userId) {
    ... on UserOnline {
      user { id name }
      timestamp
    }
    ... on UserOffline {
      user { id name }
      timestamp
      lastSeen
    }
    ... on UserTyping {
      user { id name }
      channelId
    }
  }
}
```

```vue
<script setup lang="ts">
import { matchUnion } from '@bgql/client';

useSubscription(OnUserEventDocument, {
  variables: { userId: currentUserId },
  onData: (data) => {
    matchUnion(data.userEvent, {
      UserOnline: (event) => {
        setUserOnline(event.user.id);
      },
      UserOffline: (event) => {
        setUserOffline(event.user.id, event.lastSeen);
      },
      UserTyping: (event) => {
        showTypingIndicator(event.user, event.channelId);
      },
    });
  },
});
</script>
```

### Updating Cache

```typescript
useSubscription(PostUpdatedDocument, {
  variables: { authorId: userId },
  onData: (data, { cache }) => {
    // Update cache with new data
    cache.writeFragment({
      id: `Post:${data.postUpdated.id}`,
      fragment: PostFragmentDoc,
      data: data.postUpdated,
    });
  },
});
```

## Presence System

### Online Users

```vue
<script setup lang="ts">
import { ref, computed } from 'vue';
import { useSubscription } from '@bgql/client/vue';

const onlineUsers = ref(new Set<string>());

useSubscription(PresenceDocument, {
  variables: { roomId: 'room-1' },
  onData: (data) => {
    matchUnion(data.presence, {
      UserJoined: (event) => {
        onlineUsers.value.add(event.user.id);
      },
      UserLeft: (event) => {
        onlineUsers.value.delete(event.user.id);
      },
    });
  },
});

const onlineCount = computed(() => onlineUsers.value.size);
</script>

<template>
  <div>{{ onlineCount }} users online</div>
</template>
```

### Typing Indicators

```vue
<script setup lang="ts">
const typingUsers = ref(new Map<string, NodeJS.Timeout>());

useSubscription(TypingDocument, {
  variables: { channelId },
  onData: (data) => {
    const { user, isTyping } = data.typing;

    if (isTyping) {
      // Clear existing timeout
      const existing = typingUsers.value.get(user.id);
      if (existing) clearTimeout(existing);

      // Set new timeout to clear after 3s
      const timeout = setTimeout(() => {
        typingUsers.value.delete(user.id);
      }, 3000);

      typingUsers.value.set(user.id, timeout);
    } else {
      const timeout = typingUsers.value.get(user.id);
      if (timeout) clearTimeout(timeout);
      typingUsers.value.delete(user.id);
    }
  },
});

const typingNames = computed(() => {
  return Array.from(typingUsers.value.keys())
    .filter(id => id !== currentUser.id)
    .map(id => getUserName(id));
});
</script>

<template>
  <div v-if="typingNames.length > 0">
    {{ typingNames.join(', ') }} {{ typingNames.length === 1 ? 'is' : 'are' }} typing...
  </div>
</template>
```

## Connection Management

### Connection State

```typescript
import { useSubscriptionClient } from '@bgql/client/vue';

const { connected, connecting, error, reconnect } = useSubscriptionClient();
```

```vue
<template>
  <div v-if="connecting" class="status connecting">
    Connecting...
  </div>
  <div v-else-if="!connected" class="status disconnected">
    Disconnected
    <button @click="reconnect">Reconnect</button>
  </div>
  <div v-else class="status connected">
    Connected
  </div>
</template>
```

### Manual Reconnection

```typescript
const client = createClient('http://localhost:4000/graphql', {
  subscriptions: {
    url: 'ws://localhost:4000/subscriptions',
    reconnect: true,
    onReconnected: () => {
      // Refetch queries to sync state
      client.refetchQueries();
    },
  },
});
```

### Cleanup on Unmount

```vue
<script setup lang="ts">
import { onUnmounted } from 'vue';
import { useSubscription } from '@bgql/client/vue';

const { stop } = useSubscription(MessageCreatedDocument, {
  variables: { channelId: 'channel-1' },
  onData: handleMessage,
});

// Automatically stops on unmount, but can be manual
onUnmounted(() => {
  stop();
});
</script>
```

## Error Handling

### Subscription Errors

```typescript
useSubscription(MessageCreatedDocument, {
  variables: { channelId: 'channel-1' },
  onData: handleMessage,
  onError: (error) => {
    if (error.code === 'UNAUTHORIZED') {
      router.push('/login');
    } else {
      showError('Connection error. Retrying...');
    }
  },
});
```

### Reconnection Handling

```typescript
const client = createClient('http://localhost:4000/graphql', {
  subscriptions: {
    url: 'ws://localhost:4000/subscriptions',
    reconnect: true,
    reconnectAttempts: 5,
    onReconnecting: (attempt) => {
      console.log(`Reconnecting... attempt ${attempt}`);
    },
    onReconnectFailed: () => {
      showError('Failed to reconnect. Please refresh the page.');
    },
  },
});
```

## Best Practices

### 1. Unsubscribe When Not Needed

```vue
<script setup lang="ts">
// Good: Conditional subscription
const { data } = useSubscription(NotificationsDocument, {
  enabled: () => isLoggedIn.value && isVisible.value,
});

// Automatically unsubscribes when disabled
</script>
```

### 2. Batch Updates

```typescript
const pendingMessages = ref<Message[]>([]);
let flushTimeout: NodeJS.Timeout;

useSubscription(MessageCreatedDocument, {
  variables: { channelId },
  onData: (data) => {
    pendingMessages.value.push(data.messageCreated);

    // Batch updates
    clearTimeout(flushTimeout);
    flushTimeout = setTimeout(() => {
      messages.value.push(...pendingMessages.value);
      pendingMessages.value = [];
    }, 100);
  },
});
```

### 3. Handle Offline State

```typescript
useSubscription(MessageCreatedDocument, {
  variables: { channelId },
  enabled: () => navigator.onLine,
  onData: handleMessage,
});

// Listen for online/offline
window.addEventListener('online', () => {
  // Refetch missed messages
  refetchMessages();
});
```

### 4. Limit Subscription Scope

```graphql
# Good: Subscribe to specific channel
subscription OnMessage($channelId: ID!) {
  messageCreated(channelId: $channelId) {
    id
    content
  }
}

# Avoid: Subscribe to all messages
subscription OnAllMessages {
  messageCreated {
    id
    content
  }
}
```

## Next Steps

- [Backend Subscriptions](/backend/subscriptions)
- [Queries](/frontend/queries)
- [Caching](/frontend/caching)
