---
layout: home

hero:
  name: Better GraphQL
  text: Type-Safe API Development
  tagline: A modern GraphQL implementation with Rust-inspired features, strict type safety, and zero-config SDK
  actions:
    - theme: brand
      text: Get Started
      link: /guide/getting-started
    - theme: alt
      text: View on GitHub
      link: https://github.com/ubugeeei/bgql

features:
  - icon: 🦀
    title: Rust-Inspired Type System
    details: "Option&lt;T&gt;, List&lt;T&gt;, generics with constraints, opaque types, and a powerful module system inspired by Rust."
  - icon: 🔒
    title: Type-Safe by Default
    details: Full type inference from schema to client. TypedDocumentNode, discriminated unions, and strict null checking.
  - icon: ⚡
    title: High Performance
    details: Written in Rust with SIMD optimizations. Zero-copy parsing, arena allocation, and parallel execution.
  - icon: 🛠️
    title: Zero-Config SDK
    details: One-liner server setup, automatic query batching, normalized caching, and Vue/React composables.
  - icon: 📝
    title: Excellent DX
    details: Full LSP support with completions, hover, go-to-definition, diagnostics, and code actions.
  - icon: 🌊
    title: Streaming Support
    details: Native @defer and @stream support for incremental data delivery.
---

## Quick Example

::: code-group

```graphql [schema.bgql]
interface Node {
  id: ID
}

type User implements Node {
  id: ID
  name: String
  email: String @requireAuth
  posts: List<Post>
}

type Post implements Node {
  id: ID
  title: String
  content: Option<String>
  author: User
}

union UserResult = User | NotFoundError | UnauthorizedError

type Query {
  user(id: ID): UserResult
  users(first: Int = 10): Connection<User>
}
```

```typescript [server.ts]
import { serve, defineResolvers } from '@bgql/server'

const resolvers = defineResolvers({
  Query: {
    user: async (_, { id }, ctx) => {
      const user = await ctx.db.users.findById(id)
      if (!user) return { __typename: 'NotFoundError', message: 'User not found' }
      return { __typename: 'User', ...user }
    }
  }
})

serve({ schema: './schema.bgql', resolvers })
```

```typescript [client.ts]
import { createClient, gql } from '@bgql/client'

const client = createClient('http://localhost:4000/graphql')

const GetUser = gql`
  query GetUser($id: ID!) {
    user(id: $id) {
      ... on User {
        id
        name
      }
      ... on NotFoundError {
        message
      }
    }
  }
`

// Full type inference!
const result = await client.execute(GetUser, { id: '1' })
```

:::
