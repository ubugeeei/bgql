# Why Better GraphQL?

Better GraphQL (bgql) addresses the pain points of traditional GraphQL while maintaining full compatibility with the GraphQL ecosystem.

## Problems with Traditional GraphQL

### 1. Nullable by Default

In traditional GraphQL, all fields are nullable by default. This leads to defensive coding:

```typescript
// Traditional GraphQL - everything might be null
const name = user?.profile?.name ?? 'Unknown'
const posts = user?.posts ?? []
```

**Better GraphQL Solution:** Fields are non-null by default. Use `Option<T>` to explicitly mark nullable fields:

```graphql
type User {
  id: ID           # Required - never null
  name: String     # Required - never null
  bio: Option<String>  # Explicitly optional
}
```

### 2. No Generic Types

Traditional GraphQL forces you to repeat pagination types:

```graphql
# Traditional - lots of duplication
type UserConnection {
  edges: [UserEdge]
  pageInfo: PageInfo
}
type UserEdge {
  node: User
  cursor: String
}

type PostConnection {
  edges: [PostEdge]
  pageInfo: PageInfo
}
# ... repeated for every type
```

**Better GraphQL Solution:** Generic types with constraints:

```graphql
interface Node {
  id: ID
}

# Define once, use everywhere
type Connection<T extends Node> {
  edges: List<Edge<T>>
  pageInfo: PageInfo
  totalCount: Int
}

type Query {
  users: Connection<User>
  posts: Connection<Post>
  comments: Connection<Comment>
}
```

### 3. Weak Error Handling

Traditional GraphQL puts errors in a separate array, losing type information:

```json
{
  "data": { "user": null },
  "errors": [{ "message": "User not found" }]
}
```

**Better GraphQL Solution:** Errors are first-class types:

```graphql
type NotFoundError {
  message: String
  resourceId: ID
}

type ValidationError {
  message: String
  field: String
}

union CreateUserResult = User | ValidationError | EmailAlreadyExistsError

type Mutation {
  createUser(input: CreateUserInput): CreateUserResult
}
```

This enables exhaustive pattern matching in the client:

```typescript
const result = await client.execute(CreateUser, { input })

switch (result.createUser.__typename) {
  case 'User':
    console.log('Created:', result.createUser.name)
    break
  case 'ValidationError':
    showFieldError(result.createUser.field, result.createUser.message)
    break
  case 'EmailAlreadyExistsError':
    showError('Email already registered')
    break
}
```

### 4. No Module System

Large GraphQL schemas become unwieldy in a single file.

**Better GraphQL Solution:** Rust-inspired module system:

```graphql
# schema/mod.bgql
mod users;
mod posts;
mod common;

use::common::{PageInfo, Connection}
use::users::User
use::posts::Post

schema {
  query: Query
  mutation: Mutation
}
```

```graphql
# schema/users/mod.bgql
pub type User implements Node {
  id: ID
  name: String
}

pub input CreateUserInput {
  name: String
  email: String
}
```

### 5. Inconsistent Validation

Validation is typically scattered across resolvers.

**Better GraphQL Solution:** Built-in validation directives:

```graphql
input CreateUserInput {
  name: String @minLength(1) @maxLength(100)
  email: String @email
  password: String @minLength(8) @pattern(regex: "^(?=.*[A-Za-z])(?=.*\\d).+$")
  age: Int @min(0) @max(150)
  website: Option<String> @url
}
```

## Performance

Better GraphQL is written in Rust with performance as a priority:

| Feature | Benefit |
|---------|---------|
| Zero-copy parsing | Minimal memory allocation during parsing |
| Arena allocation | Batch deallocation for parsed AST |
| SIMD operations | Accelerated string searching |
| Parallel execution | Rayon-powered resolver execution |
| Query planning | Optimal execution order |

## Comparison

| Feature | GraphQL | Better GraphQL |
|---------|---------|----------------|
| Nullable fields | Default | Explicit `Option<T>` |
| Generic types | ❌ | ✅ With constraints |
| Module system | ❌ | ✅ Rust-inspired |
| Error types | Separate array | First-class unions |
| Validation | Manual | Built-in directives |
| Type inference | Limited | Full end-to-end |
| Streaming | Extensions | Native `@defer`/`@stream` |

## Migration from GraphQL

Better GraphQL is designed to be incrementally adoptable:

1. **Schema Migration**: Your existing GraphQL schema works with minor modifications
2. **Client Compatibility**: Standard GraphQL clients can query bgql servers
3. **Gradual Adoption**: Adopt new features (generics, modules, etc.) as needed

See the [Migration Guide](/guide/migration) for detailed steps.
