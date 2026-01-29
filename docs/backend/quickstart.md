# Backend Quick Start

This guide will help you set up a Better GraphQL server in minutes.

## Installation

```bash
npm install @bgql/server
```

## Project Structure

```
my-api/
├── schema/
│   ├── mod.bgql          # Main schema file
│   ├── users.bgql        # User types
│   └── posts.bgql        # Post types
├── resolvers/
│   ├── index.ts          # Resolver exports
│   ├── users.ts          # User resolvers
│   └── posts.ts          # Post resolvers
├── server.ts             # Server entry point
└── package.json
```

## Basic Server

### 1. Define Your Schema

```graphql
# schema/mod.bgql
interface Node {
  id: ID
}

type User implements Node {
  id: ID
  name: String
  email: String
}

type Query {
  user(id: ID): Option<User>
  users: List<User>
}

type Mutation {
  createUser(name: String, email: String): User
}
```

### 2. Write Resolvers

```typescript
// resolvers/index.ts
import { defineResolvers } from '@bgql/server'

// In-memory database for demo
const users = new Map<string, { id: string; name: string; email: string }>()

export const resolvers = defineResolvers({
  Query: {
    user: (_, { id }) => users.get(id) ?? null,
    users: () => Array.from(users.values()),
  },

  Mutation: {
    createUser: (_, { name, email }) => {
      const id = crypto.randomUUID()
      const user = { id, name, email }
      users.set(id, user)
      return user
    },
  },
})
```

### 3. Start the Server

```typescript
// server.ts
import { serve } from '@bgql/server'
import { resolvers } from './resolvers'

serve({
  schema: './schema/mod.bgql',
  resolvers,
  port: 4000,
})

console.log('Server running at http://localhost:4000/graphql')
```

### 4. Run

```bash
npx tsx server.ts
```

That's it! Your GraphQL server is running.

## Configuration Options

```typescript
serve({
  // Schema
  schema: './schema/mod.bgql',
  resolvers,

  // Server
  port: 4000,
  host: '0.0.0.0',

  // Features
  playground: true,        // Enable GraphQL Playground
  introspection: true,     // Enable introspection

  // CORS
  cors: {
    origin: ['http://localhost:3000'],
    credentials: true,
  },

  // Context
  context: (req) => ({
    user: authenticateUser(req),
    db: database,
  }),

  // Middleware
  middleware: [
    loggingMiddleware,
    rateLimitMiddleware,
  ],
})
```

## Zero-Config Defaults

The `serve()` function provides sensible defaults:

| Feature | Default |
|---------|---------|
| Port | 4000 |
| Playground | Enabled in development |
| Introspection | Enabled in development |
| CORS | Same-origin |
| Body limit | 100kb |
| Query depth | 10 |
| Query complexity | 1000 |

## Development Mode

For development with hot reload:

```typescript
import { devServer } from '@bgql/server'

devServer({
  schema: './schema/mod.bgql',
  resolvers,
  watch: true,  // Watch for schema changes
})
```

Or use the CLI:

```bash
bgql dev --schema ./schema/mod.bgql
```

## Testing

```typescript
import { createTestClient } from '@bgql/server/testing'
import { resolvers } from './resolvers'

describe('User API', () => {
  const client = createTestClient({
    schema: './schema/mod.bgql',
    resolvers,
  })

  it('creates a user', async () => {
    const result = await client.execute(`
      mutation {
        createUser(name: "John", email: "john@example.com") {
          id
          name
        }
      }
    `)

    expect(result.data.createUser.name).toBe('John')
  })
})
```

## Next Steps

- [Resolvers in Depth](/backend/resolvers)
- [Context and Authentication](/backend/context)
- [Error Handling](/backend/errors)
- [DataLoader Pattern](/backend/dataloader)
