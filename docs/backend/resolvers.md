# Resolvers

Resolvers are functions that produce the data for your schema fields.

## Basic Structure

```typescript
import { defineResolvers } from '@bgql/server'

const resolvers = defineResolvers({
  Query: {
    // Root query resolvers
  },
  Mutation: {
    // Root mutation resolvers
  },
  Subscription: {
    // Root subscription resolvers
  },
  // Type resolvers
  User: {
    // Field resolvers for User type
  },
})
```

## Resolver Arguments

Every resolver receives four arguments:

```typescript
const resolvers = defineResolvers({
  Query: {
    user: (parent, args, context, info) => {
      // parent: The parent object (null for root resolvers)
      // args: The arguments passed to the field
      // context: Shared context across all resolvers
      // info: GraphQL execution info
    }
  }
})
```

### Typed Resolvers

For full type safety, use the generated types:

```typescript
import { Resolvers } from './generated/types'

const resolvers: Resolvers = {
  Query: {
    user: (_, { id }, ctx) => {
      // id is typed as string
      // return type is typed as User | null
    }
  }
}
```

## Query Resolvers

```typescript
const resolvers = defineResolvers({
  Query: {
    // Simple query
    hello: () => 'Hello, World!',

    // With arguments
    user: (_, { id }) => db.users.findById(id),

    // Async resolver
    users: async (_, { first, after }) => {
      return await db.users.paginate({ first, after })
    },

    // With context
    me: (_, __, { user }) => user,
  }
})
```

## Mutation Resolvers

```typescript
const resolvers = defineResolvers({
  Mutation: {
    createUser: async (_, { input }, { db }) => {
      const user = await db.users.create(input)
      return user
    },

    updateUser: async (_, { id, input }, { db, user }) => {
      // Authorization check
      if (user.id !== id && !user.isAdmin) {
        throw new ForbiddenError('Cannot update other users')
      }
      return db.users.update(id, input)
    },

    deleteUser: async (_, { id }, { db }) => {
      await db.users.delete(id)
      return true
    },
  }
})
```

## Field Resolvers

When a type has fields that need custom resolution:

```typescript
const resolvers = defineResolvers({
  User: {
    // Computed field
    fullName: (user) => `${user.firstName} ${user.lastName}`,

    // Related data
    posts: (user, _, { db }) => db.posts.findByAuthor(user.id),

    // With arguments
    friends: (user, { first }, { db }) => {
      return db.users.findFriends(user.id, { limit: first })
    },
  },

  Post: {
    author: (post, _, { db }) => db.users.findById(post.authorId),

    // Optional field
    publishedAt: (post) => post.publishedAt ?? null,
  }
})
```

## Result Types (Union Resolvers)

For union types representing different outcomes:

```typescript
// Schema:
// union UserResult = User | NotFoundError | UnauthorizedError

const resolvers = defineResolvers({
  Query: {
    user: async (_, { id }, { db, user: currentUser }) => {
      // Check authentication
      if (!currentUser) {
        return {
          __typename: 'UnauthorizedError',
          message: 'Authentication required',
        }
      }

      // Find user
      const user = await db.users.findById(id)
      if (!user) {
        return {
          __typename: 'NotFoundError',
          message: 'User not found',
          resourceType: 'User',
          resourceId: id,
        }
      }

      return {
        __typename: 'User',
        ...user,
      }
    }
  }
})
```

## Interface Resolvers

Resolve the concrete type for interfaces:

```typescript
const resolvers = defineResolvers({
  Node: {
    __resolveType: (obj) => {
      if ('email' in obj) return 'User'
      if ('title' in obj) return 'Post'
      return null
    }
  }
})
```

## Async/Parallel Execution

Resolvers can return Promises and are executed in parallel when possible:

```typescript
const resolvers = defineResolvers({
  Query: {
    dashboard: async (_, __, { db }) => {
      // These execute in parallel
      const [users, posts, stats] = await Promise.all([
        db.users.count(),
        db.posts.count(),
        db.analytics.getStats(),
      ])

      return { users, posts, stats }
    }
  }
})
```

## Resolver Helpers

### createResolver

Type-safe resolver factory:

```typescript
import { createResolver } from '@bgql/server'

const getUser = createResolver<{ id: string }, User | null>(
  async (_, { id }, { db }) => {
    return db.users.findById(id)
  }
)

const resolvers = defineResolvers({
  Query: {
    user: getUser,
  }
})
```

### withAuth

Authentication wrapper:

```typescript
import { withAuth } from '@bgql/server'

const resolvers = defineResolvers({
  Query: {
    me: withAuth((_, __, { user }) => user),

    adminDashboard: withAuth(
      (_, __, { db }) => db.analytics.getAdminStats(),
      { role: 'ADMIN' }
    ),
  }
})
```

### withValidation

Input validation wrapper:

```typescript
import { withValidation } from '@bgql/server'

const resolvers = defineResolvers({
  Mutation: {
    createUser: withValidation(
      {
        name: { minLength: 1, maxLength: 100 },
        email: { email: true },
      },
      async (_, { input }, { db }) => {
        return db.users.create(input)
      }
    ),
  }
})
```

## Best Practices

### 1. Keep Resolvers Thin

Delegate business logic to services:

```typescript
// ❌ Bad: Business logic in resolver
const resolvers = defineResolvers({
  Mutation: {
    createOrder: async (_, { input }, { db }) => {
      const items = await db.products.findByIds(input.itemIds)
      const total = items.reduce((sum, item) => sum + item.price, 0)
      if (total > input.maxBudget) throw new Error('Over budget')
      // ... more logic
    }
  }
})

// ✅ Good: Delegate to service
const resolvers = defineResolvers({
  Mutation: {
    createOrder: (_, { input }, { orderService }) => {
      return orderService.createOrder(input)
    }
  }
})
```

### 2. Use DataLoader for N+1

See [DataLoader Guide](/backend/dataloader) for details.

### 3. Handle Errors Properly

See [Error Handling Guide](/backend/errors) for details.

## Next Steps

- [Context and Authentication](/backend/context)
- [DataLoader Pattern](/backend/dataloader)
- [Error Handling](/backend/errors)
