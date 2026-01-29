# Vue.js Integration

Better GraphQL provides first-class Vue 3 support with composables and components.

## Setup

### 1. Install

```bash
npm install @bgql/client vue
```

### 2. Configure the Plugin

```typescript
// main.ts
import { createApp } from 'vue'
import { BgqlPlugin, createClient } from '@bgql/client/vue'
import App from './App.vue'

const client = createClient({
  url: 'http://localhost:4000/graphql',
  headers: () => ({
    'Authorization': `Bearer ${localStorage.getItem('token')}`,
  }),
})

const app = createApp(App)
app.use(BgqlPlugin, { client })
app.mount('#app')
```

## Composables

### useQuery

Fetch data reactively:

```vue
<script setup lang="ts">
import { useQuery } from '@bgql/client/vue'
import { GetUsersDocument } from './generated/graphql'

const { data, loading, error, refetch } = useQuery(GetUsersDocument, {
  variables: { first: 10 },
})
</script>

<template>
  <div v-if="loading" class="loading">Loading users...</div>

  <div v-else-if="error" class="error">
    Error: {{ error.message }}
    <button @click="refetch()">Retry</button>
  </div>

  <ul v-else class="user-list">
    <li v-for="edge in data?.users.edges" :key="edge.node.id">
      {{ edge.node.name }}
    </li>
  </ul>
</template>
```

#### Options

```typescript
const { data, loading, error, refetch, fetchMore } = useQuery(GetUsers, {
  // Variables (can be reactive)
  variables: computed(() => ({
    first: pageSize.value,
    after: cursor.value,
  })),

  // Fetch policy
  fetchPolicy: 'cache-first', // 'cache-first' | 'network-only' | 'cache-only'

  // Poll interval (ms)
  pollInterval: 5000,

  // Skip query
  skip: computed(() => !isLoggedIn.value),

  // Callbacks
  onCompleted: (data) => console.log('Loaded:', data),
  onError: (error) => console.error('Failed:', error),
})
```

#### Pagination

```vue
<script setup lang="ts">
import { useQuery } from '@bgql/client/vue'

const { data, loading, fetchMore } = useQuery(GetUsers, {
  variables: { first: 10 },
})

async function loadMore() {
  if (!data.value?.users.pageInfo.hasNextPage) return

  await fetchMore({
    variables: {
      first: 10,
      after: data.value.users.pageInfo.endCursor,
    },
    updateQuery: (prev, { fetchMoreResult }) => ({
      users: {
        ...fetchMoreResult.users,
        edges: [...prev.users.edges, ...fetchMoreResult.users.edges],
      },
    }),
  })
}
</script>

<template>
  <ul>
    <li v-for="edge in data?.users.edges" :key="edge.node.id">
      {{ edge.node.name }}
    </li>
  </ul>

  <button
    v-if="data?.users.pageInfo.hasNextPage"
    @click="loadMore"
    :disabled="loading"
  >
    Load More
  </button>
</template>
```

### useMutation

Execute mutations:

```vue
<script setup lang="ts">
import { ref } from 'vue'
import { useMutation } from '@bgql/client/vue'
import { CreateUserDocument } from './generated/graphql'

const name = ref('')
const email = ref('')

const { mutate, loading, error } = useMutation(CreateUserDocument, {
  onCompleted: (data) => {
    console.log('Created:', data.createUser.id)
    name.value = ''
    email.value = ''
  },
})

async function handleSubmit() {
  await mutate({
    input: {
      name: name.value,
      email: email.value,
    },
  })
}
</script>

<template>
  <form @submit.prevent="handleSubmit">
    <input v-model="name" placeholder="Name" required />
    <input v-model="email" type="email" placeholder="Email" required />

    <div v-if="error" class="error">{{ error.message }}</div>

    <button type="submit" :disabled="loading">
      {{ loading ? 'Creating...' : 'Create User' }}
    </button>
  </form>
</template>
```

#### Optimistic Updates

```typescript
const { mutate } = useMutation(UpdateUserDocument, {
  optimisticResponse: (variables) => ({
    updateUser: {
      __typename: 'User',
      id: variables.id,
      name: variables.input.name,
    },
  }),

  update: (cache, { data }) => {
    // Update cache after mutation
    cache.modify({
      id: cache.identify(data.updateUser),
      fields: {
        name: () => data.updateUser.name,
      },
    })
  },
})
```

### useSubscription

Real-time updates:

```vue
<script setup lang="ts">
import { ref } from 'vue'
import { useSubscription } from '@bgql/client/vue'
import { OnMessageDocument } from './generated/graphql'

const messages = ref<Message[]>([])

const { data } = useSubscription(OnMessageDocument, {
  variables: { channelId: props.channelId },
  onData: ({ data }) => {
    messages.value.push(data.messageCreated)
  },
})
</script>

<template>
  <div class="chat">
    <div v-for="msg in messages" :key="msg.id" class="message">
      <strong>{{ msg.author.name }}:</strong>
      {{ msg.content }}
    </div>
  </div>
</template>
```

### useLazyQuery

Manual query execution:

```vue
<script setup lang="ts">
import { useLazyQuery } from '@bgql/client/vue'

const { execute, data, loading } = useLazyQuery(SearchUsersDocument)

const searchTerm = ref('')

async function handleSearch() {
  await execute({ query: searchTerm.value })
}
</script>

<template>
  <div class="search">
    <input v-model="searchTerm" @keyup.enter="handleSearch" />
    <button @click="handleSearch" :disabled="loading">Search</button>

    <ul v-if="data">
      <li v-for="user in data.searchUsers" :key="user.id">
        {{ user.name }}
      </li>
    </ul>
  </div>
</template>
```

## Components

### BgqlProvider

Provide client to component tree:

```vue
<script setup lang="ts">
import { BgqlProvider, createClient } from '@bgql/client/vue'

const client = createClient('http://localhost:4000/graphql')
</script>

<template>
  <BgqlProvider :client="client">
    <router-view />
  </BgqlProvider>
</template>
```

### BgqlDefer

Handle deferred data:

```vue
<script setup lang="ts">
import { BgqlDefer } from '@bgql/client/vue'
</script>

<template>
  <div class="user-profile">
    <h1>{{ user.name }}</h1>

    <!-- Deferred content -->
    <BgqlDefer :data="user.posts" #default="{ data: posts }">
      <template #loading>
        <div class="skeleton">Loading posts...</div>
      </template>

      <ul class="posts">
        <li v-for="post in posts" :key="post.id">
          {{ post.title }}
        </li>
      </ul>
    </BgqlDefer>
  </div>
</template>
```

## Type Safety

### Generated Types

Generate types from your schema:

```bash
bgql codegen --lang typescript schema.bgql -o ./src/generated/graphql.ts
```

The generated file includes:

```typescript
// Types
export interface User {
  __typename: 'User'
  id: string
  name: string
  email: string
}

// Document nodes
export const GetUsersDocument: TypedDocumentNode<
  GetUsersQuery,
  GetUsersQueryVariables
>

// Query/Mutation types
export interface GetUsersQuery {
  users: {
    edges: Array<{ node: User }>
    pageInfo: PageInfo
  }
}
```

### Typed Composables

When using generated documents, everything is typed:

```vue
<script setup lang="ts">
import { useQuery } from '@bgql/client/vue'
import { GetUserDocument } from './generated/graphql'

const { data } = useQuery(GetUserDocument, {
  variables: { id: '1' }, // Typed!
})

// data.value?.user is typed as User | NotFoundError | null
if (data.value?.user.__typename === 'User') {
  console.log(data.value.user.name) // Typed!
}
</script>
```

## Error Handling

### Global Error Handler

```typescript
app.use(BgqlPlugin, {
  client,
  onError: (error, operation) => {
    if (error.message.includes('Unauthorized')) {
      router.push('/login')
    }
  },
})
```

### Per-Query Error Handling

```vue
<script setup lang="ts">
const { data, error } = useQuery(GetUser, {
  onError: (error) => {
    if (error.networkError) {
      toast.error('Network error. Please check your connection.')
    }
  },
})
</script>

<template>
  <div v-if="error?.graphQLErrors?.length">
    <div v-for="err in error.graphQLErrors" class="error">
      {{ err.message }}
    </div>
  </div>
</template>
```

## Suspense Support

```vue
<script setup lang="ts">
import { useQuery } from '@bgql/client/vue'

// With suspense: true, the query throws a promise for Suspense
const { data } = useQuery(GetUser, {
  variables: { id: props.id },
  suspense: true,
})
</script>

<template>
  <!-- data is guaranteed to be defined here -->
  <div>{{ data.user.name }}</div>
</template>
```

Parent component:

```vue
<template>
  <Suspense>
    <template #default>
      <UserProfile :id="userId" />
    </template>
    <template #fallback>
      <LoadingSpinner />
    </template>
  </Suspense>
</template>
```

## Best Practices

### 1. Colocate Queries

Keep queries close to the components that use them:

```
components/
  UserProfile/
    UserProfile.vue
    UserProfile.graphql    # Query definition
    UserProfile.types.ts   # Generated types
```

### 2. Use Fragments

Share field selections:

```graphql
fragment UserFields on User {
  id
  name
  email
  avatarUrl
}

query GetUser($id: ID!) {
  user(id: $id) {
    ...UserFields
  }
}

query GetUsers {
  users {
    edges {
      node {
        ...UserFields
      }
    }
  }
}
```

### 3. Prefetch Critical Data

```typescript
// In router beforeEnter
router.beforeEach(async (to) => {
  if (to.name === 'user-profile') {
    await client.prefetch(GetUserDocument, { id: to.params.id })
  }
})
```

## Next Steps

- [Caching](/frontend/caching)
- [Subscriptions](/frontend/subscriptions)
- [Streaming with @defer](/frontend/streaming)
