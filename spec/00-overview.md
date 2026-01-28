# Better GraphQL Specification - Overview and Design Principles

**Version**: 0.0.0 (Draft)

## 1. Introduction

Better GraphQL is a new query language specification designed to inherit GraphQL's design philosophy while solving challenges in modern web development.

### 1.1 GraphQL's Strengths We Keep

Better GraphQL inherits these core design principles from GraphQL:

- **Graph-based Data Model**: Data is modeled as a graph where types are nodes and relationships between types are edges. This enables natural traversal of interconnected data through a single query, reflecting how application data is actually structured
- **Declarative Data Fetching**: Clients specify exactly what data they need by describing the shape of the response, not how to fetch it
- **Type System**: Schema-based contract-driven development with strong typing
- **Single Endpoint**: Fetch multiple resources and traverse relationships in a single request, eliminating the need for multiple round-trips
- **Introspection**: Self-describing schemas that enable powerful tooling and documentation

#### Graph Theory Foundation

The "Graph" in GraphQL refers to the graph data structure from computer science. In this model:

- **Nodes**: Each type in the schema (User, Post, Comment, etc.) represents a node
- **Edges**: Fields that reference other types represent edges connecting nodes
- **Traversal**: Queries traverse the graph starting from root nodes (Query, Mutation, Subscription)

```graphql
# The schema defines a graph structure
type User {
  id: ID
  name: String
  posts: List<Post>      # Edge: User -> Post
  followers: List<User>  # Edge: User -> User (self-referential)
}

type Post {
  id: ID
  title: String
  author: User       # Edge: Post -> User
  comments: List<Comment> # Edge: Post -> Comment
}

type Comment {
  id: ID
  content: String
  author: User       # Edge: Comment -> User
  post: Post         # Edge: Comment -> Post
}
```

```graphql
# A query traverses this graph
query {
  user(id: "1") {           # Start at User node
    name
    posts {                  # Traverse User -> Post edge
      title
      comments {             # Traverse Post -> Comment edge
        content
        author {             # Traverse Comment -> User edge
          name
        }
      }
    }
    followers {              # Traverse User -> User edge
      name
    }
  }
}
```

This graph-based approach provides several advantages:

1. **Natural Data Modeling**: Reflects how entities relate to each other in the real world
2. **Flexible Queries**: Clients can traverse any path through the graph they need
3. **Efficient Data Loading**: Related data can be fetched in a single request
4. **Discoverable API**: The graph structure is self-documenting through introspection

### 1.2 Problems Better GraphQL Solves

| Problem | GraphQL Status | Better GraphQL Solution |
|---------|---------------|------------------------|
| Nullable by default | `String` is nullable, `String!` is non-null | `String` is non-null, `String?` is nullable |
| Error handling | `errors` array with no type info | Typed union errors |
| Input Union | Not supported | Fully supported |
| Date/Time types | Custom scalars per project | Built-in `Date`, `DateTime` |
| Validation | Not in schema | Declarative directives |
| HTTP integration | Headers/Cookies outside spec | First-class citizens |
| Streaming | `@defer`/`@stream` experimental | Priority-based, officially supported |

## 2. Design Principles

### 2.1 Maximize Type Safety

Better GraphQL aims to catch as many errors as possible at compile time.

```graphql
# Bad: GraphQL - null checks required
type User {
  name: String  # nullable
}

# Good: Better GraphQL - non-null by default
type User {
  name: String          # non-null
  nickname: Option<String>  # nullable
}
```

### 2.2 Natural HTTP Integration

Better GraphQL treats HTTP protocol as a first-class citizen.

```graphql
type Query {
  # Direct access to HTTP headers and cookies
  currentUser: User @requireAuth
}

# CORS configuration in schema
schema @cors(
  origins: ["https://example.com"],
  credentials: true
) {
  query: Query
}
```

### 2.3 Streaming First

Streaming is a standard feature to minimize network latency.

```graphql
query {
  user(id: "1") {
    name
    posts @stream(initialCount: 5, priority: 1) {
      title
      content @defer(priority: 2)
    }
  }
}
```

### 2.4 Developer Experience (DX) Focus

- **Clear Error Messages**: Show problem location and solution
- **Enhanced Introspection**: Validation rules are also queryable
- **Server-side Fragments**: Server-managed reusable field sets

### 2.5 Immutability by Default

All generated types are immutable by default. This prevents accidental mutations and enables safe sharing of data structures.

```typescript
// Generated types use readonly modifiers
interface User {
  readonly id: UserId;
  readonly name: string;
  readonly email: string;
  readonly posts: ReadonlyArray<Post>;
}

// Arrays are also readonly
type UserList = ReadonlyArray<User>;

// Attempting to mutate causes compile error
const user: User = await client.getUser({ id });
user.name = "Changed";  // Error: Cannot assign to 'name' because it is a read-only property

// To modify, create a new object
const updatedUser = { ...user, name: "New Name" };
```

This principle applies throughout:
- **Schema-generated types**: All object types are readonly
- **Query results**: All returned data is immutable
- **Configuration objects**: Client/server configs are readonly
- **Error types**: Error data cannot be mutated

## 3. Compatibility with GraphQL

### 3.1 Migration Path

Better GraphQL supports gradual migration from GraphQL.

1. **Phase 1**: Convert existing GraphQL schema to Better GraphQL
   - `String!` → `String`
   - `String` → `Option<String>`

2. **Phase 2**: Gradually introduce new features
   - Add typed errors
   - Add validation directives

3. **Phase 3**: Full migration
   - Leverage HTTP integration
   - Optimize streaming

### 3.2 Compatibility Mode

Better GraphQL servers can operate in GraphQL compatibility mode:

```yaml
# better-graphql.config.yaml
compatibility:
  mode: graphql  # 'graphql' | 'better-graphql' | 'hybrid'
  nullable_default: true  # GraphQL-compatible nullable default
```

## 4. Specification Structure

| File | Contents |
|------|----------|
| `01-type-system.md` | Type System |
| `02-schema-definition.md` | Schema Definition Language |
| `03-directives.md` | Directives |
| `04-query-language.md` | Query Language |
| `05-http-protocol.md` | HTTP Protocol |
| `06-execution.md` | Execution Model |
| `07-introspection.md` | Introspection |

## 5. Versioning Policy

Better GraphQL follows Semantic Versioning:

- **MAJOR**: Breaking changes
- **MINOR**: Backward-compatible features
- **PATCH**: Bug fixes

### 5.1 RFC Process

New features are added through an RFC (Request for Comments) process:

1. Submit RFC document
2. Community review period (minimum 30 days)
3. Experimental implementation verification
4. Official adoption

## 6. Glossary

| Term | Definition |
|------|------------|
| **Field** | A property of an object type |
| **Resolver** | A function that resolves a field's value |
| **Fragment** | A reusable set of fields |
| **Directive** | Metadata attached to schema or queries |
| **Introspection** | Self-describing schema queries |

## 7. Conventions

This specification uses the following conventions:

- `MUST` / `MUST NOT`: Absolute requirements
- `SHOULD` / `SHOULD NOT`: Strong recommendations
- `MAY`: Optional

Code examples use the following extensions:
- `.bgql`: Better GraphQL schema/query files
