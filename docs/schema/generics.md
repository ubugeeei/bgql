# Generic Types

Better GraphQL supports generic types with constraints, inspired by TypeScript and Rust.

## Basic Generics

Define reusable type patterns:

```graphql
# Generic edge type
type Edge<T> {
  node: T
  cursor: String
}

# Generic connection type
type Connection<T> {
  edges: List<Edge<T>>
  pageInfo: PageInfo
  totalCount: Int
}

# Use them
type Query {
  users: Connection<User>
  posts: Connection<Post>
  comments: Connection<Comment>
}
```

## Type Parameters

### Single Parameter

```graphql
type Box<T> {
  value: T
}

type Response<T> {
  data: Option<T>
  error: Option<String>
}
```

### Multiple Parameters

```graphql
type Pair<A, B> {
  first: A
  second: B
}

type Result<T, E> {
  value: Option<T>
  error: Option<E>
}
```

## Constraints

Constrain type parameters to implement specific interfaces:

```graphql
interface Node {
  id: ID
}

# T must implement Node
type Connection<T extends Node> {
  edges: List<Edge<T>>
  pageInfo: PageInfo
  totalCount: Int
}
```

### Multiple Constraints

```graphql
interface Node {
  id: ID
}

interface Timestamped {
  createdAt: DateTime
}

# T must implement both Node and Timestamped
type AuditedConnection<T extends Node & Timestamped> {
  edges: List<Edge<T>>
  pageInfo: PageInfo
  lastModified: DateTime
}
```

### Constraint Validation

The type checker validates constraints at use sites:

```graphql
interface Node {
  id: ID
}

type Connection<T extends Node> {
  edges: List<Edge<T>>
}

type User implements Node {
  id: ID
  name: String
}

type TempData {
  value: String
}

type Query {
  users: Connection<User>      # ✅ Valid: User implements Node
  temp: Connection<TempData>   # ❌ Error: TempData doesn't implement Node
}
```

## Generic Interfaces

Interfaces can also be generic:

```graphql
interface Repository<T extends Node> {
  findById(id: ID): Option<T>
  findAll(first: Int): List<T>
  count: Int
}

type UserRepository implements Repository<User> {
  findById(id: ID): Option<User>
  findAll(first: Int): List<User>
  count: Int
}
```

## Nested Generics

Generics can be nested:

```graphql
type Response<T> {
  data: Option<T>
  errors: List<Error>
}

type Query {
  # Response containing a Connection of Users
  users: Response<Connection<User>>

  # Option containing a List of Items
  items: Option<List<Item>>
}
```

## Common Patterns

### Pagination

```graphql
interface Node {
  id: ID
}

type PageInfo {
  hasNextPage: Boolean
  hasPreviousPage: Boolean
  startCursor: Option<String>
  endCursor: Option<String>
}

type Edge<T> {
  node: T
  cursor: String
}

type Connection<T extends Node> {
  edges: List<Edge<T>>
  pageInfo: PageInfo
  totalCount: Int
}

# Usage
type Query {
  users(first: Int, after: Option<String>): Connection<User>
  posts(first: Int, after: Option<String>): Connection<Post>
}
```

### Result Type

```graphql
interface Error {
  message: String
  code: String
}

type Success<T> {
  data: T
}

type Failure<E extends Error> {
  error: E
}

# Union combining success and failure
union Result<T, E extends Error> = Success<T> | Failure<E>

# Usage with specific error types
type NotFoundError implements Error {
  message: String
  code: String
  resourceType: String
  resourceId: ID
}

type Query {
  user(id: ID): Result<User, NotFoundError>
}
```

### Payload Pattern

```graphql
type Payload<T> {
  data: Option<T>
  success: Boolean
  message: Option<String>
}

type Mutation {
  createUser(input: CreateUserInput): Payload<User>
  updateUser(id: ID, input: UpdateUserInput): Payload<User>
  deleteUser(id: ID): Payload<Boolean>
}
```

### Batch Operations

```graphql
type BatchResult<T> {
  succeeded: List<T>
  failed: List<BatchError>
}

type BatchError {
  index: Int
  error: String
}

type Mutation {
  createUsers(inputs: List<CreateUserInput>): BatchResult<User>
  deleteUsers(ids: List<ID>): BatchResult<ID>
}
```

## Type Inference

Generic types are inferred in queries:

```graphql
# Schema
type Connection<T extends Node> {
  edges: List<Edge<T>>
}

type Query {
  users: Connection<User>
}

# Query - no need to specify generic parameter
query {
  users {
    edges {
      node {
        id    # Inferred as User.id
        name  # Inferred as User.name
      }
    }
  }
}
```

## TypeScript Generation

Generic types generate proper TypeScript:

```graphql
# Schema
type Connection<T extends Node> {
  edges: List<Edge<T>>
  pageInfo: PageInfo
}
```

```typescript
// Generated TypeScript
interface Connection<T extends Node> {
  edges: Edge<T>[];
  pageInfo: PageInfo;
}

// Concrete types are also generated
type UserConnection = Connection<User>;
type PostConnection = Connection<Post>;
```

## Limitations

1. **No Higher-Kinded Types**: Cannot pass generic types as parameters
   ```graphql
   # Not supported
   type Transform<F<_>, T> {
     value: F<T>
   }
   ```

2. **No Default Type Parameters**: Must always specify type arguments
   ```graphql
   # Not supported
   type Container<T = String> {
     value: T
   }
   ```

3. **Constraints Must Be Interfaces**: Cannot use types or unions as constraints
   ```graphql
   # Not supported
   type Wrapper<T extends User> {
     value: T
   }
   ```

## Best Practices

### 1. Use Meaningful Constraints

```graphql
# ✅ Good: Clear constraint
type Repository<T extends Node> {
  findById(id: ID): Option<T>
}

# ❌ Avoid: No constraint when needed
type Repository<T> {
  findById(id: ID): Option<T>  # T might not have an id!
}
```

### 2. Keep Generic Types Simple

```graphql
# ✅ Good: Simple, focused generic
type Edge<T> {
  node: T
  cursor: String
}

# ❌ Avoid: Too many parameters
type ComplexType<A, B, C, D, E> {
  ...
}
```

### 3. Document Type Parameters

```graphql
"""
Paginated connection following Relay specification.
@typeParam T - The node type, must implement Node interface
"""
type Connection<T extends Node> {
  edges: List<Edge<T>>
  pageInfo: PageInfo
}
```

## Next Steps

- [Interfaces](/schema/interfaces)
- [Module System](/schema/modules)
- [Directives](/schema/directives)
