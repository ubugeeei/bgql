# Interfaces

Interfaces define contracts that types must implement.

## Basic Interface

```graphql
interface Node {
  id: ID
}

type User implements Node {
  id: ID
  name: String
  email: String
}

type Post implements Node {
  id: ID
  title: String
  content: String
}
```

## Interface Fields

All fields in an interface must be implemented by types:

```graphql
interface Timestamped {
  createdAt: DateTime
  updatedAt: Option<DateTime>
}

type User implements Timestamped {
  id: ID
  name: String
  createdAt: DateTime        # Required from interface
  updatedAt: Option<DateTime> # Required from interface
}
```

## Multiple Interfaces

Types can implement multiple interfaces:

```graphql
interface Node {
  id: ID
}

interface Timestamped {
  createdAt: DateTime
  updatedAt: Option<DateTime>
}

interface Authorable {
  author: User
}

# Implements all three interfaces
type Post implements Node & Timestamped & Authorable {
  id: ID
  title: String
  content: String
  author: User
  createdAt: DateTime
  updatedAt: Option<DateTime>
}
```

## Interface Inheritance

Interfaces can extend other interfaces:

```graphql
interface Node {
  id: ID
}

interface Entity extends Node {
  createdAt: DateTime
  updatedAt: Option<DateTime>
}

# Must implement all fields from both interfaces
type User implements Entity {
  id: ID
  name: String
  createdAt: DateTime
  updatedAt: Option<DateTime>
}
```

## Generic Interfaces

Interfaces can have type parameters:

```graphql
interface Node {
  id: ID
}

interface Repository<T extends Node> {
  findById(id: ID): Option<T>
  findAll(first: Int): List<T>
  count: Int
}

type User implements Node {
  id: ID
  name: String
}

# Concrete implementation
type UserRepository implements Repository<User> {
  findById(id: ID): Option<User>
  findAll(first: Int): List<User>
  count: Int
}
```

## Interface in Unions

Interfaces can be used in unions:

```graphql
interface Error {
  message: String
  code: String
}

type NotFoundError implements Error {
  message: String
  code: String
  resourceType: String
  resourceId: ID
}

type ValidationError implements Error {
  message: String
  code: String
  field: String
  value: String
}

union Result = User | NotFoundError | ValidationError
```

## Field Arguments in Interfaces

Interface fields can have arguments:

```graphql
interface Pageable<T extends Node> {
  items(first: Int, after: Option<String>): Connection<T>
  totalCount: Int
}

type UserList implements Pageable<User> {
  items(first: Int, after: Option<String>): Connection<User>
  totalCount: Int
}
```

## Interface Query Fields

Query for interface types:

```graphql
type Query {
  # Returns any type implementing Node
  node(id: ID): Option<Node>

  # Returns list of any Timestamped types
  recentItems(first: Int): List<Timestamped>
}
```

### Querying Interface Types

```graphql
query GetNode($id: ID!) {
  node(id: $id) {
    id  # Common field from Node interface

    # Type-specific fields
    ... on User {
      name
      email
    }
    ... on Post {
      title
      content
    }
  }
}
```

## TypeScript Generation

Interfaces generate TypeScript interfaces:

```graphql
interface Node {
  id: ID
}

interface Timestamped {
  createdAt: DateTime
}

type User implements Node & Timestamped {
  id: ID
  name: String
  createdAt: DateTime
}
```

```typescript
// Generated TypeScript
interface Node {
  readonly id: string;
}

interface Timestamped {
  readonly createdAt: string;
}

interface User extends Node, Timestamped {
  readonly __typename: 'User';
  readonly id: string;
  readonly name: string;
  readonly createdAt: string;
}
```

## Best Practices

### 1. Use Interfaces for Shared Behavior

```graphql
# ✅ Good: Common behavior in interface
interface Searchable {
  searchableText: String
}

type User implements Searchable {
  id: ID
  name: String
  searchableText: String  # Derived from name + bio
}

type Post implements Searchable {
  id: ID
  title: String
  content: String
  searchableText: String  # Derived from title + content
}
```

### 2. Keep Interfaces Focused

```graphql
# ✅ Good: Single responsibility
interface Node {
  id: ID
}

interface Timestamped {
  createdAt: DateTime
  updatedAt: Option<DateTime>
}

interface Authorable {
  author: User
}

# ❌ Avoid: Too many unrelated fields
interface Everything {
  id: ID
  createdAt: DateTime
  author: User
  tags: List<String>
  metadata: JSON
}
```

### 3. Use Marker Interfaces for Capabilities

```graphql
# ✅ Good: Empty interface as capability marker
interface Persistable {}
interface Cacheable {}
interface Auditable {}

type User implements Node & Persistable & Cacheable & Auditable {
  id: ID
  name: String
}
```

## Next Steps

- [Enums & Unions](/schema/enums-unions)
- [Generics](/schema/generics)
- [Types](/schema/types)
