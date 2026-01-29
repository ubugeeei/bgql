# Getting Started

Better GraphQL (bgql) is a modern GraphQL implementation with Rust-inspired features, strict type safety, and excellent developer experience.

## Installation

### CLI

```bash
# Install the CLI globally
cargo install bgql

# Or with npm
npm install -g @bgql/cli
```

### Server SDK (Node.js)

```bash
npm install @bgql/server
```

### Client SDK

```bash
npm install @bgql/client
```

## Your First Schema

Create a file named `schema.bgql`:

```graphql
# Define interfaces for shared fields
interface Node {
  id: ID
}

interface Timestamped {
  createdAt: DateTime
  updatedAt: Option<DateTime>
}

# Define your types
type User implements Node & Timestamped {
  id: ID
  name: String
  email: String
  createdAt: DateTime
  updatedAt: Option<DateTime>
  posts: List<Post>
}

type Post implements Node & Timestamped {
  id: ID
  title: String
  content: String
  author: User
  createdAt: DateTime
  updatedAt: Option<DateTime>
}

# Define error types
type NotFoundError {
  message: String
  resourceType: String
  resourceId: ID
}

# Use union for result types
union UserResult = User | NotFoundError

# Define queries
type Query {
  user(id: ID): UserResult
  users(first: Int = 10, after: Option<String>): Connection<User>
}

# Define mutations
type Mutation {
  createUser(input: CreateUserInput): User
}

# Define input types
input CreateUserInput {
  name: String @minLength(1) @maxLength(100)
  email: String @email
}
```

## Start the Server

Create `server.ts`:

```typescript
import { serve, defineResolvers } from '@bgql/server'

const resolvers = defineResolvers({
  Query: {
    user: async (_, { id }, ctx) => {
      const user = await ctx.db.users.findById(id)
      if (!user) {
        return {
          __typename: 'NotFoundError',
          message: 'User not found',
          resourceType: 'User',
          resourceId: id
        }
      }
      return { __typename: 'User', ...user }
    },
    users: async (_, { first, after }, ctx) => {
      return ctx.db.users.paginate({ first, after })
    }
  },
  Mutation: {
    createUser: async (_, { input }, ctx) => {
      return ctx.db.users.create(input)
    }
  }
})

// Start the server with one line!
serve({
  schema: './schema.bgql',
  resolvers,
  context: (req) => ({
    db: createDatabase(),
    user: req.headers.authorization ? verifyToken(req.headers.authorization) : null
  })
})
```

Run the server:

```bash
npx tsx server.ts
```

Your GraphQL server is now running at `http://localhost:4000/graphql`!

## Query from the Client

Create `client.ts`:

```typescript
import { createClient, gql } from '@bgql/client'

// Create client with just the URL
const client = createClient('http://localhost:4000/graphql')

// Define a typed query
const GetUser = gql`
  query GetUser($id: ID!) {
    user(id: $id) {
      ... on User {
        id
        name
        email
        posts {
          id
          title
        }
      }
      ... on NotFoundError {
        message
      }
    }
  }
`

// Execute with full type inference
async function main() {
  const result = await client.execute(GetUser, { id: '1' })

  if (result.user.__typename === 'User') {
    console.log(`Hello, ${result.user.name}!`)
    console.log(`Posts: ${result.user.posts.length}`)
  } else {
    console.error(result.user.message)
  }
}

main()
```

## Generate Types

For even better type safety, generate TypeScript types from your schema:

```bash
bgql codegen --lang typescript schema.bgql -o ./generated/types.ts
```

Then import and use the generated types:

```typescript
import { GetUserQuery, GetUserQueryVariables } from './generated/types'

// Now fully typed!
const result = await client.execute<GetUserQuery, GetUserQueryVariables>(
  GetUser,
  { id: '1' }
)
```

## Next Steps

- [Learn about the Type System](/guide/type-system)
- [Set up the Backend](/backend/quickstart)
- [Set up the Frontend](/frontend/quickstart)
- [Explore the CLI](/cli/overview)
