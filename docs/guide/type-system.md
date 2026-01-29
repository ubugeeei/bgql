# Type System

Better GraphQL's type system is inspired by Rust, providing explicit nullability, generics, and powerful type inference.

## Explicit Nullability

Unlike traditional GraphQL where all fields are nullable by default, Better GraphQL requires explicit nullability.

### Non-Null (Default)

```graphql
type User {
  id: ID        # Required - never null
  name: String  # Required - never null
  email: String # Required - never null
}
```

### Optional with `Option<T>`

```graphql
type User {
  id: ID
  name: String
  bio: Option<String>           # Can be null
  avatarUrl: Option<String>     # Can be null
  deletedAt: Option<DateTime>   # Can be null
}
```

### Why This Matters

```typescript
// Traditional GraphQL - defensive coding required
const displayName = user?.name ?? 'Unknown';
const bio = user?.bio ?? '';

// Better GraphQL - type system guarantees
const displayName = user.name;  // Always defined
const bio = user.bio ?? '';     // Only bio can be null
```

## List Types

### `List<T>` - List of Non-Null Items

```graphql
type User {
  tags: List<String>      # ["a", "b", "c"] - no nulls in array
  friends: List<User>     # All items are User, never null
}
```

### Combined with Option

```graphql
type User {
  # The list itself is optional
  nickname: Option<List<String>>

  # List of optional items (some might be null)
  partialResults: List<Option<Item>>

  # Optional list of optional items
  maybePartial: Option<List<Option<Item>>>
}
```

## Generic Types

Better GraphQL supports generics with constraints, enabling reusable type patterns.

### Basic Generics

```graphql
type Edge<T> {
  node: T
  cursor: String
}

type Connection<T> {
  edges: List<Edge<T>>
  pageInfo: PageInfo
  totalCount: Int
}

type Query {
  users: Connection<User>
  posts: Connection<Post>
}
```

### Constrained Generics

```graphql
interface Node {
  id: ID
}

# T must implement Node
type Connection<T extends Node> {
  edges: List<Edge<T>>
  pageInfo: PageInfo
}

type User implements Node {
  id: ID
  name: String
}

# Valid: User implements Node
type Query {
  users: Connection<User>
}
```

## Type Aliases

Create type aliases for common patterns:

```graphql
# Type alias
type alias UserConnection = Connection<User>
type alias PostConnection = Connection<Post>

type Query {
  users: UserConnection
  posts: PostConnection
}
```

## Opaque Types (Newtypes)

Create distinct types from existing types:

```graphql
# Opaque types - distinct at type level
opaque Email = String
opaque UserId = ID
opaque PostId = ID

type User {
  id: UserId      # Not interchangeable with PostId
  email: Email    # Not interchangeable with String
}

type Post {
  id: PostId      # Different type than UserId
  authorId: UserId
}
```

### TypeScript Generation

```typescript
// Generated branded types
type Email = string & { readonly __brand: 'Email' };
type UserId = string & { readonly __brand: 'UserId' };
type PostId = string & { readonly __brand: 'PostId' };

// Compile-time safety
function getUser(id: UserId): User { ... }

const userId: UserId = '123' as UserId;
const postId: PostId = '456' as PostId;

getUser(userId);  // ✅ OK
getUser(postId);  // ❌ Type error: PostId not assignable to UserId
```

## Interfaces

Interfaces define contracts that types must implement:

```graphql
interface Node {
  id: ID
}

interface Timestamped {
  createdAt: DateTime
  updatedAt: Option<DateTime>
}

type User implements Node & Timestamped {
  id: ID
  name: String
  createdAt: DateTime
  updatedAt: Option<DateTime>
}
```

## Union Types

Unions represent values that can be one of several types:

```graphql
type User {
  id: ID
  name: String
}

type NotFoundError {
  message: String
}

type AuthError {
  message: String
}

union UserResult = User | NotFoundError | AuthError
```

### Discriminated Unions

All union members have `__typename` for type discrimination:

```typescript
const result = await client.query(GetUser, { id: '1' });

switch (result.user.__typename) {
  case 'User':
    console.log(result.user.name);
    break;
  case 'NotFoundError':
  case 'AuthError':
    console.log(result.user.message);
    break;
}
```

## Enums

### Simple Enums

```graphql
enum UserRole {
  ADMIN
  MODERATOR
  USER
  GUEST
}
```

### Enums with Data (Rust-style)

```graphql
enum Result {
  Ok(String)
  Err { code: Int, message: String }
}

enum Shape {
  Circle { radius: Float }
  Rectangle { width: Float, height: Float }
  Point
}
```

## Input Types

Input types define mutation/query arguments:

```graphql
input CreateUserInput {
  name: String
  email: String
  role: UserRole = USER  # Default value
}

input UpdateUserInput {
  name: Option<String>   # Optional update
  email: Option<String>
}
```

## Type Inference

Better GraphQL provides full end-to-end type inference:

```typescript
// Schema
type Query {
  user(id: ID): UserResult
}

union UserResult = User | NotFoundError

// Generated TypeScript
const GetUserDocument: TypedDocumentNode<
  { user: User | NotFoundError },
  { id: string }
>;

// Full inference in client code
const result = await client.query(GetUserDocument, { id: '1' });
//    ^? Result<{ user: User | NotFoundError }, ClientError>

if (result.ok && result.value.user.__typename === 'User') {
  console.log(result.value.user.name);
  //                        ^? string (inferred)
}
```

## Comparison with GraphQL

| Feature | GraphQL | Better GraphQL |
|---------|---------|----------------|
| Nullability | Nullable by default | Non-null by default |
| Optional | `field: Type` | `field: Option<Type>` |
| Required | `field: Type!` | `field: Type` |
| Generics | ❌ | ✅ With constraints |
| Opaque types | ❌ | ✅ Nominal typing |
| Enum data | ❌ | ✅ Rust-style |
| Type aliases | ❌ | ✅ |

## Next Steps

- [Module System](/guide/modules)
- [Generics](/schema/generics)
- [Interfaces](/schema/interfaces)
