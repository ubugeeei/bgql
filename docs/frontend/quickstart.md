# Frontend Quick Start

This guide will help you set up the Better GraphQL client in your frontend application.

## Installation

```bash
npm install @bgql/client
```

For Vue.js integration:

```bash
npm install @bgql/client vue
```

## Basic Usage

### 1. Create the Client

```typescript
import { createClient } from '@bgql/client'

// Simple: just provide the URL
const client = createClient('http://localhost:4000/graphql')

// With options
const client = createClient({
  url: 'http://localhost:4000/graphql',
  headers: {
    'Authorization': `Bearer ${token}`,
  },
})
```

### 2. Define Queries

```typescript
import { gql } from '@bgql/client'

const GetUser = gql`
  query GetUser($id: ID!) {
    user(id: $id) {
      ... on User {
        id
        name
        email
      }
      ... on NotFoundError {
        message
      }
    }
  }
`

const GetUsers = gql`
  query GetUsers($first: Int) {
    users(first: $first) {
      edges {
        node {
          id
          name
        }
      }
      pageInfo {
        hasNextPage
        endCursor
      }
    }
  }
`
```

### 3. Execute Queries

```typescript
// Execute a query
const result = await client.execute(GetUser, { id: '1' })

// Handle the result
if (result.user.__typename === 'User') {
  console.log(`Hello, ${result.user.name}!`)
} else {
  console.error(result.user.message)
}
```

### 4. Execute Mutations

```typescript
const CreateUser = gql`
  mutation CreateUser($input: CreateUserInput!) {
    createUser(input: $input) {
      id
      name
    }
  }
`

const result = await client.execute(CreateUser, {
  input: {
    name: 'John Doe',
    email: 'john@example.com',
  },
})

console.log('Created user:', result.createUser.id)
```

## Type-Safe Queries

For full type safety, generate types from your schema:

```bash
bgql codegen --lang typescript schema.bgql -o ./generated/types.ts
```

Then use `TypedDocumentNode`:

```typescript
import type { TypedDocumentNode } from '@bgql/client'
import type { GetUserQuery, GetUserQueryVariables } from './generated/types'

const GetUser: TypedDocumentNode<GetUserQuery, GetUserQueryVariables> = gql`
  query GetUser($id: ID!) {
    user(id: $id) {
      ... on User {
        id
        name
        email
      }
      ... on NotFoundError {
        message
      }
    }
  }
`

// Now fully typed!
const result = await client.execute(GetUser, { id: '1' })
// result.user is typed as User | NotFoundError
```

## Client Options

```typescript
const client = createClient({
  // Required
  url: 'http://localhost:4000/graphql',

  // Authentication
  headers: {
    'Authorization': `Bearer ${token}`,
  },

  // Or dynamic headers
  headers: () => ({
    'Authorization': `Bearer ${getToken()}`,
  }),

  // Request options
  timeout: 30000,
  credentials: 'include',

  // Retry configuration
  retry: {
    maxRetries: 3,
    initialDelayMs: 1000,
    maxDelayMs: 30000,
  },

  // Performance features
  dedupe: true,        // Deduplicate identical requests
  batch: true,         // Batch multiple queries
  cache: true,         // Enable caching

  // Error handling
  onError: (error) => {
    console.error('GraphQL Error:', error)
    reportToSentry(error)
  },
})
```

## Middleware

Add custom logic to all requests:

```typescript
const client = createClient({
  url: 'http://localhost:4000/graphql',
  middleware: [
    // Logging middleware
    async (operation, options, next) => {
      console.log('Executing:', operation.operationName)
      const start = Date.now()

      const result = await next(operation, options)

      console.log(`Completed in ${Date.now() - start}ms`)
      return result
    },

    // Auth refresh middleware
    async (operation, options, next) => {
      try {
        return await next(operation, options)
      } catch (error) {
        if (error.message.includes('Unauthorized')) {
          await refreshToken()
          return next(operation, options)
        }
        throw error
      }
    },
  ],
})
```

## Subscriptions

For real-time updates:

```typescript
const UserCreated = gql`
  subscription OnUserCreated {
    userCreated {
      id
      name
    }
  }
`

const subscription = client.subscribe(UserCreated)

subscription.subscribe({
  next: (data) => {
    console.log('New user:', data.userCreated.name)
  },
  error: (error) => {
    console.error('Subscription error:', error)
  },
  complete: () => {
    console.log('Subscription completed')
  },
})

// Later: unsubscribe
subscription.unsubscribe()
```

## Framework Integration

### Vue.js

```typescript
import { createApp } from 'vue'
import { BgqlPlugin } from '@bgql/client/vue'

const app = createApp(App)

app.use(BgqlPlugin, {
  client: createClient('http://localhost:4000/graphql'),
})

app.mount('#app')
```

Then in components:

```vue
<script setup lang="ts">
import { useQuery } from '@bgql/client/vue'

const { data, loading, error, refetch } = useQuery(GetUsers, {
  variables: { first: 10 },
})
</script>

<template>
  <div v-if="loading">Loading...</div>
  <div v-else-if="error">Error: {{ error.message }}</div>
  <ul v-else>
    <li v-for="edge in data.users.edges" :key="edge.node.id">
      {{ edge.node.name }}
    </li>
  </ul>
</template>
```

See the [Vue.js Guide](/frontend/vue) for more details.

## Error Handling

```typescript
import { BgqlError, NetworkError, ValidationError } from '@bgql/client'

try {
  const result = await client.execute(GetUser, { id: '1' })
} catch (error) {
  if (error instanceof NetworkError) {
    console.error('Network error:', error.message)
  } else if (error instanceof ValidationError) {
    console.error('Validation error:', error.field, error.message)
  } else if (error instanceof BgqlError) {
    console.error('GraphQL error:', error.message)
  }
}
```

## Next Steps

- [Queries in Depth](/frontend/queries)
- [Mutations](/frontend/mutations)
- [Type Safety](/frontend/type-safety)
- [Vue.js Integration](/frontend/vue)
- [Caching](/frontend/caching)
