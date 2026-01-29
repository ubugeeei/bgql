# Client Error Handling

Handle errors gracefully in your Better GraphQL client applications.

## Error Types

### Network Errors

Connection failures, timeouts, and other transport-level errors:

```typescript
const result = await client.query(GetUserDocument, { id: '1' });

if (!result.ok) {
  switch (result.error.type) {
    case 'network':
      // Connection failed
      showOfflineMessage();
      break;
    case 'timeout':
      // Request timed out
      showTimeoutMessage();
      break;
    case 'abort':
      // Request was cancelled
      break;
  }
}
```

### GraphQL Errors

Server-returned errors in the response:

```typescript
if (!result.ok && result.error.type === 'graphql') {
  const { message, path, extensions } = result.error;

  if (extensions?.code === 'UNAUTHENTICATED') {
    redirectToLogin();
  } else if (extensions?.code === 'FORBIDDEN') {
    showPermissionError();
  } else {
    showGenericError(message);
  }
}
```

### Domain Errors

Business logic errors returned as union types:

```typescript
if (result.ok) {
  matchUnion(result.value.user, {
    User: (user) => showUser(user),
    NotFoundError: (error) => showNotFound(error.resourceId),
    AuthError: (error) => showAuthError(error.message),
  });
}
```

## Result Pattern

### Using Result Helpers

```typescript
import { isOk, isErr, unwrap, unwrapOr, match } from '@bgql/client';

const result = await client.query(GetUserDocument, { id: '1' });

// Type guards
if (isOk(result)) {
  console.log(result.value);  // Data type
}

if (isErr(result)) {
  console.log(result.error);  // Error type
}

// Unwrap (throws on error)
try {
  const data = unwrap(result);
} catch (e) {
  handleError(e);
}

// Unwrap with default
const data = unwrapOr(result, { user: null });

// Pattern matching
const message = match(result, {
  ok: (data) => `Hello, ${data.user.name}!`,
  err: (error) => `Error: ${error.message}`,
});
```

### Chaining Results

```typescript
import { mapResult, flatMapResult } from '@bgql/client';

const result = await client.query(GetUserDocument, { id: '1' });

// Transform success value
const nameResult = mapResult(result, (data) => data.user.name);

// Chain async operations
const postsResult = await flatMapResult(result, async (data) => {
  return client.query(GetPostsDocument, { authorId: data.user.id });
});
```

## Vue Error Handling

### Error State in Composables

```vue
<script setup lang="ts">
import { useQuery } from '@bgql/client/vue';
import { GetUserDocument } from './generated/graphql';

const { data, loading, error } = useQuery(GetUserDocument, {
  variables: { id: '1' },
});
</script>

<template>
  <div v-if="loading">Loading...</div>
  <div v-else-if="error" class="error">
    <p>{{ error.message }}</p>
    <button @click="refetch">Retry</button>
  </div>
  <UserProfile v-else :user="data.user" />
</template>
```

### Error Boundary Component

```vue
<!-- ErrorBoundary.vue -->
<script setup lang="ts">
import { ref, onErrorCaptured } from 'vue';

const error = ref<Error | null>(null);

onErrorCaptured((err) => {
  error.value = err;
  return false;  // Don't propagate
});

function reset() {
  error.value = null;
}
</script>

<template>
  <div v-if="error" class="error-boundary">
    <h2>Something went wrong</h2>
    <p>{{ error.message }}</p>
    <button @click="reset">Try again</button>
  </div>
  <slot v-else />
</template>
```

```vue
<!-- Usage -->
<template>
  <ErrorBoundary>
    <UserProfile :id="userId" />
  </ErrorBoundary>
</template>
```

### Global Error Handler

```typescript
// main.ts
import { createClient } from '@bgql/client';
import { createApp } from 'vue';

const client = createClient('http://localhost:4000/graphql', {
  onError: (error) => {
    if (error.type === 'network') {
      toast.error('Network error. Please check your connection.');
    } else if (error.extensions?.code === 'UNAUTHENTICATED') {
      router.push('/login');
    }
  },
});

const app = createApp(App);
app.provide('graphqlClient', client);
```

## React Error Handling

### Error State in Hooks

```tsx
import { useQuery } from '@bgql/client/react';

function UserProfile({ id }: { id: string }) {
  const { data, loading, error, refetch } = useQuery(GetUserDocument, {
    variables: { id },
  });

  if (loading) return <Spinner />;

  if (error) {
    return (
      <div className="error">
        <p>{error.message}</p>
        <button onClick={() => refetch()}>Retry</button>
      </div>
    );
  }

  return <User user={data.user} />;
}
```

### Error Boundary

```tsx
import { ErrorBoundary } from '@bgql/client/react';

function App() {
  return (
    <ErrorBoundary
      fallback={({ error, resetError }) => (
        <div className="error-page">
          <h1>Something went wrong</h1>
          <p>{error.message}</p>
          <button onClick={resetError}>Try again</button>
        </div>
      )}
    >
      <UserProfile id="1" />
    </ErrorBoundary>
  );
}
```

### Query Error Boundary

```tsx
import { QueryErrorBoundary } from '@bgql/client/react';

function UserSection({ id }: { id: string }) {
  return (
    <QueryErrorBoundary
      fallback={({ error }) => <UserError error={error} />}
      onReset={() => {
        // Called when error boundary resets
      }}
    >
      <UserProfile id={id} />
      <UserPosts id={id} />
    </QueryErrorBoundary>
  );
}
```

## Form Error Handling

### Mutation Errors

```vue
<script setup lang="ts">
import { ref } from 'vue';
import { useMutation, matchUnion } from '@bgql/client/vue';
import { CreateUserDocument } from './generated/graphql';

const errors = ref<Record<string, string>>({});
const generalError = ref<string | null>(null);

const { mutate, loading } = useMutation(CreateUserDocument, {
  onCompleted: (data) => {
    matchUnion(data.createUser, {
      User: (user) => {
        router.push(`/users/${user.id}`);
      },
      ValidationError: (error) => {
        errors.value[error.field] = error.message;
      },
      ValidationErrors: ({ errors: validationErrors }) => {
        for (const error of validationErrors) {
          errors.value[error.field] = error.message;
        }
      },
    });
  },
  onError: (error) => {
    generalError.value = 'An unexpected error occurred. Please try again.';
  },
});

async function handleSubmit(input: CreateUserInput) {
  errors.value = {};
  generalError.value = null;
  await mutate({ input });
}
</script>

<template>
  <form @submit.prevent="handleSubmit(formData)">
    <div v-if="generalError" class="alert error">
      {{ generalError }}
    </div>

    <div class="field">
      <input v-model="formData.name" placeholder="Name" />
      <span v-if="errors.name" class="error">{{ errors.name }}</span>
    </div>

    <div class="field">
      <input v-model="formData.email" type="email" placeholder="Email" />
      <span v-if="errors.email" class="error">{{ errors.email }}</span>
    </div>

    <button type="submit" :disabled="loading">
      {{ loading ? 'Creating...' : 'Create User' }}
    </button>
  </form>
</template>
```

### Field-Level Validation

```typescript
import { useForm } from '@bgql/client/vue';

const { register, errors, handleSubmit, setError } = useForm<CreateUserInput>();

const { mutate } = useMutation(CreateUserDocument, {
  onCompleted: (data) => {
    matchUnion(data.createUser, {
      User: (user) => router.push(`/users/${user.id}`),
      ValidationError: (error) => {
        setError(error.field as keyof CreateUserInput, error.message);
      },
    });
  },
});
```

## Retry Logic

### Automatic Retry

```typescript
const client = createClient('http://localhost:4000/graphql', {
  retry: {
    attempts: 3,
    delay: 1000,
    // Only retry network errors
    shouldRetry: (error) => error.type === 'network',
  },
});
```

### Manual Retry

```vue
<script setup lang="ts">
const { data, error, loading, refetch } = useQuery(GetUserDocument, {
  variables: { id: '1' },
});

async function retryWithBackoff(attempts = 3) {
  for (let i = 0; i < attempts; i++) {
    const result = await refetch();
    if (result.ok) return result;

    // Exponential backoff
    await new Promise(r => setTimeout(r, Math.pow(2, i) * 1000));
  }
  throw new Error('Max retries exceeded');
}
</script>
```

## Offline Handling

### Detecting Offline State

```typescript
import { useOnlineStatus } from '@bgql/client/vue';

const { isOnline, wasOffline } = useOnlineStatus();

// Show offline indicator
// Queue mutations when offline
// Retry when back online
```

### Offline Queue

```typescript
const client = createClient('http://localhost:4000/graphql', {
  offline: {
    enabled: true,
    storage: localStorage,
    // Queue mutations when offline
    queueMutations: true,
    // Retry when back online
    retryOnReconnect: true,
  },
});

// Check pending mutations
const pending = client.offline.getPendingMutations();
```

## Logging and Monitoring

### Error Logging

```typescript
const client = createClient('http://localhost:4000/graphql', {
  onError: (error, operation) => {
    // Log to monitoring service
    Sentry.captureException(error, {
      extra: {
        operationName: operation.operationName,
        variables: operation.variables,
      },
    });
  },
});
```

### Request Logging

```typescript
const client = createClient('http://localhost:4000/graphql')
  .use(loggingMiddleware({
    logErrors: true,
    logQueries: process.env.NODE_ENV === 'development',
  }));
```

## Best Practices

### 1. Always Handle Both Error Types

```typescript
// Network errors (result.ok === false)
if (!result.ok) {
  handleNetworkError(result.error);
  return;
}

// Domain errors (part of successful response)
matchUnion(result.value.user, {
  User: handleSuccess,
  NotFoundError: handleNotFound,
  AuthError: handleAuth,
});
```

### 2. Provide Actionable Error Messages

```typescript
function getErrorMessage(error: Error): string {
  switch (error.extensions?.code) {
    case 'UNAUTHENTICATED':
      return 'Please log in to continue.';
    case 'FORBIDDEN':
      return 'You do not have permission to perform this action.';
    case 'NOT_FOUND':
      return 'The requested resource was not found.';
    case 'VALIDATION_ERROR':
      return 'Please check your input and try again.';
    default:
      return 'An unexpected error occurred. Please try again later.';
  }
}
```

### 3. Use Error Boundaries Strategically

```vue
<template>
  <!-- Page-level boundary for critical errors -->
  <ErrorBoundary>
    <Layout>
      <!-- Section-level boundaries for isolated failures -->
      <ErrorBoundary>
        <UserProfile />
      </ErrorBoundary>

      <ErrorBoundary>
        <UserPosts />
      </ErrorBoundary>
    </Layout>
  </ErrorBoundary>
</template>
```

### 4. Graceful Degradation

```vue
<template>
  <div>
    <UserProfile v-if="profileData" :data="profileData" />
    <UserProfileSkeleton v-else-if="profileLoading" />
    <UserProfileFallback v-else />

    <!-- Show available data even if some queries fail -->
    <UserPosts v-if="postsData" :posts="postsData.posts" />
    <div v-else-if="postsError" class="error-card">
      Failed to load posts.
      <button @click="refetchPosts">Retry</button>
    </div>
  </div>
</template>
```

## Next Steps

- [Backend Errors](/backend/errors)
- [Type Safety](/frontend/type-safety)
- [Mutations](/frontend/mutations)
