# Context and Authentication

The context object is shared across all resolvers in a request, making it perfect for authentication, database connections, and request-specific data.

## Creating Context

```typescript
import { serve } from '@bgql/server'

serve({
  schema: './schema.bgql',
  resolvers,
  context: (req) => ({
    // Database connection
    db: createDatabase(),

    // Authenticated user
    user: authenticateRequest(req),

    // Request-specific data
    requestId: req.headers['x-request-id'],
    locale: req.headers['accept-language'],
  }),
})
```

## Context Type

Define your context type for type safety:

```typescript
interface Context {
  db: Database
  user: User | null
  requestId: string
  locale: string
}

const resolvers = defineResolvers<Context>({
  Query: {
    me: (_, __, { user }) => {
      // user is typed as User | null
      return user
    },
  },
})
```

## Authentication

### JWT Authentication

```typescript
import jwt from 'jsonwebtoken'

serve({
  schema: './schema.bgql',
  resolvers,
  context: async (req) => {
    const token = req.headers.authorization?.replace('Bearer ', '')

    let user = null
    if (token) {
      try {
        const payload = jwt.verify(token, process.env.JWT_SECRET!)
        user = await db.users.findById(payload.sub)
      } catch {
        // Invalid token - user remains null
      }
    }

    return { db, user }
  },
})
```

### Session Authentication

```typescript
import { getSession } from './session'

serve({
  context: async (req) => {
    const session = await getSession(req)
    const user = session?.userId
      ? await db.users.findById(session.userId)
      : null

    return { db, user, session }
  },
})
```

## Authorization

### In Resolvers

```typescript
const resolvers = defineResolvers({
  Query: {
    adminDashboard: (_, __, { user }) => {
      if (!user) {
        throw new AuthenticationError('Must be logged in')
      }
      if (user.role !== 'ADMIN') {
        throw new ForbiddenError('Admin access required')
      }
      return getAdminStats()
    },
  },
})
```

### Using Directives

```graphql
type Query {
  me: User @requireAuth
  adminDashboard: AdminStats @hasRole(role: ADMIN)
  userProfile(id: ID): User @hasPermission(permission: "users:read")
}
```

### Authorization Helper

```typescript
import { withAuth } from '@bgql/server'

const resolvers = defineResolvers({
  Query: {
    // Requires any authenticated user
    me: withAuth((_, __, { user }) => user),

    // Requires specific role
    adminDashboard: withAuth(
      (_, __, { db }) => db.analytics.getAdminStats(),
      { role: 'ADMIN' }
    ),

    // Requires specific permission
    users: withAuth(
      (_, __, { db }) => db.users.findAll(),
      { permission: 'users:read' }
    ),
  },
})
```

## Per-Request Resources

### Request ID

```typescript
import { randomUUID } from 'crypto'

serve({
  context: (req) => ({
    requestId: req.headers['x-request-id'] || randomUUID(),
  }),
})
```

### Logging

```typescript
serve({
  context: (req) => {
    const requestId = randomUUID()
    const logger = createLogger({ requestId })

    return {
      requestId,
      logger,
    }
  },
})
```

### DataLoader

```typescript
import { createLoaders } from './loaders'

serve({
  context: (req) => {
    // Create new DataLoader instances per request
    return {
      loaders: createLoaders(db),
    }
  },
})
```

## Cleanup

For resources that need cleanup after the request:

```typescript
serve({
  context: async (req) => {
    const connection = await db.getConnection()

    return {
      db: connection,
      [Symbol.dispose]: () => {
        connection.release()
      },
    }
  },
})
```

## Best Practices

### 1. Keep Context Lean

```typescript
// ✅ Good: Only what's needed
context: (req) => ({
  user: authenticate(req),
  loaders: createLoaders(),
})

// ❌ Avoid: Heavy operations in context
context: async (req) => ({
  allUsers: await db.users.findAll(),  // Don't fetch all users!
  settings: await loadAllSettings(),    // Don't load everything!
})
```

### 2. Use Lazy Loading

```typescript
context: (req) => ({
  get user() {
    // Only authenticate when accessed
    return this._user ??= authenticate(req)
  },
})
```

### 3. Type Your Context

```typescript
// context.ts
export interface Context {
  db: Database
  user: User | null
  loaders: Loaders
  logger: Logger
}

// resolvers.ts
import { defineResolvers } from '@bgql/server'
import type { Context } from './context'

const resolvers = defineResolvers<Context>({
  // Full type safety!
})
```

## Next Steps

- [Error Handling](/backend/errors)
- [DataLoader](/backend/dataloader)
- [Testing](/backend/testing)
