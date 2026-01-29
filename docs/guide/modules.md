# Module System

Better GraphQL's module system is inspired by Rust, enabling large schemas to be organized into maintainable, reusable modules.

## Basic Modules

### File-Based Modules

```
schema/
├── mod.bgql          # Root module
├── users/
│   ├── mod.bgql      # Users module
│   └── types.bgql
├── posts/
│   ├── mod.bgql      # Posts module
│   └── types.bgql
└── common/
    └── mod.bgql      # Shared types
```

### Declaring Modules

```graphql
# schema/mod.bgql
mod users;
mod posts;
mod common;

schema {
  query: Query
  mutation: Mutation
}
```

### Module Contents

```graphql
# schema/users/mod.bgql
pub type User implements Node {
  id: ID
  name: String
  email: String
}

pub input CreateUserInput {
  name: String
  email: String
}

# Private - not exported
type UserInternal {
  passwordHash: String
}
```

## Visibility

### Public Types (`pub`)

```graphql
# users/mod.bgql
pub type User {           # Accessible from other modules
  id: ID
  name: String
}

pub input CreateUserInput {  # Accessible from other modules
  name: String
}
```

### Private Types (default)

```graphql
# users/mod.bgql
type PasswordHash {       # Only visible in this module
  hash: String
  salt: String
}

type UserSession {        # Only visible in this module
  token: String
  expiresAt: DateTime
}
```

## Imports

### Importing Types

```graphql
# posts/mod.bgql
use::users::User
use::common::{PageInfo, Connection}

pub type Post {
  id: ID
  title: String
  author: User           # From users module
}

type Query {
  posts: Connection<Post>  # From common module
}
```

### Importing with Aliases

```graphql
use::users::User as UserType
use::legacy::User as LegacyUser

type Migration {
  oldUser: LegacyUser
  newUser: UserType
}
```

### Wildcard Imports

```graphql
# Import all public types from a module
use::common::*

type Query {
  users: Connection<User>   # Connection from common
  pageInfo: PageInfo        # PageInfo from common
}
```

## Inline Modules

For smaller groupings, use inline modules:

```graphql
# schema.bgql
mod errors {
  pub type NotFoundError {
    message: String
    resourceId: ID
  }

  pub type ValidationError {
    message: String
    field: String
  }
}

use::errors::{NotFoundError, ValidationError}

union UserResult = User | NotFoundError
```

## Re-exports

Re-export types from submodules:

```graphql
# users/mod.bgql
mod types;
mod inputs;

# Re-export public types
pub use::types::User
pub use::types::UserProfile
pub use::inputs::CreateUserInput
pub use::inputs::UpdateUserInput
```

```graphql
# Other modules can now import directly
use::users::User           # Instead of users::types::User
use::users::CreateUserInput
```

## Module Organization Patterns

### Feature-Based

```
schema/
├── mod.bgql
├── auth/
│   ├── mod.bgql
│   ├── types.bgql      # User, Session, Token
│   └── mutations.bgql  # login, logout, register
├── posts/
│   ├── mod.bgql
│   ├── types.bgql      # Post, Comment
│   └── queries.bgql    # getPosts, getPost
└── common/
    ├── mod.bgql
    ├── pagination.bgql # Connection, Edge, PageInfo
    └── errors.bgql     # Error types
```

### Layer-Based

```
schema/
├── mod.bgql
├── types/
│   ├── mod.bgql
│   ├── user.bgql
│   ├── post.bgql
│   └── comment.bgql
├── inputs/
│   ├── mod.bgql
│   └── ...
├── queries/
│   ├── mod.bgql
│   └── ...
└── mutations/
    ├── mod.bgql
    └── ...
```

## Common Module

Create a common module for shared types:

```graphql
# common/mod.bgql

# Pagination
pub type PageInfo {
  hasNextPage: Boolean
  hasPreviousPage: Boolean
  startCursor: Option<String>
  endCursor: Option<String>
}

pub type Edge<T> {
  node: T
  cursor: String
}

pub type Connection<T extends Node> {
  edges: List<Edge<T>>
  pageInfo: PageInfo
  totalCount: Int
}

# Common interfaces
pub interface Node {
  id: ID
}

pub interface Timestamped {
  createdAt: DateTime
  updatedAt: Option<DateTime>
}

# Common errors
pub interface Error {
  message: String
  code: String
}

pub type NotFoundError implements Error {
  message: String
  code: String
  resourceType: String
  resourceId: ID
}
```

## Schema Entry Point

The root module defines the schema entry point:

```graphql
# mod.bgql
mod users;
mod posts;
mod common;

use::users::{User, CreateUserInput}
use::posts::{Post, CreatePostInput}
use::common::{Connection, NotFoundError}

type Query {
  user(id: ID): User | NotFoundError
  users(first: Int, after: Option<String>): Connection<User>
  post(id: ID): Post | NotFoundError
  posts(first: Int, after: Option<String>): Connection<Post>
}

type Mutation {
  createUser(input: CreateUserInput): User
  createPost(input: CreatePostInput): Post
}

schema {
  query: Query
  mutation: Mutation
}
```

## Circular Dependencies

Better GraphQL allows forward references within a module but prevents circular dependencies between modules:

```graphql
# ✅ OK: Forward reference within module
type User {
  posts: List<Post>  # Post defined below
}

type Post {
  author: User       # Back reference OK
}
```

```graphql
# ❌ Error: Circular module dependency
# users/mod.bgql
use::posts::Post     # users depends on posts

# posts/mod.bgql
use::users::User     # posts depends on users
```

### Solution: Common Module

```graphql
# common/mod.bgql
pub interface HasAuthor {
  authorId: ID
}

# users/mod.bgql
pub type User { ... }

# posts/mod.bgql
use::common::HasAuthor

pub type Post implements HasAuthor {
  authorId: ID
  # ...
}
```

## CLI Commands

### Validate Module Structure

```bash
bgql check ./schema
```

### Generate from Modules

```bash
bgql codegen ./schema --lang typescript -o ./generated
```

### Format Modules

```bash
bgql format ./schema/**/*.bgql
```

## Best Practices

### 1. Keep Modules Focused

```graphql
# ✅ Good: Single responsibility
mod users;    # User-related types
mod posts;    # Post-related types
mod auth;     # Authentication types

# ❌ Avoid: Kitchen sink modules
mod everything;  # Too broad
```

### 2. Use Clear Visibility

```graphql
# ✅ Good: Explicit visibility
pub type User { ... }      # Public API
type UserInternal { ... }  # Implementation detail
```

### 3. Minimize Exports

```graphql
# ✅ Good: Export only what's needed
pub type User
pub input CreateUserInput

# Keep helpers private
type UserValidator { ... }
```

### 4. Use Common Module for Shared Types

```graphql
# ✅ Good: Shared types in common
# common/mod.bgql
pub type PageInfo { ... }
pub type Connection<T> { ... }
pub interface Node { ... }
```

## Next Steps

- [Type System](/guide/type-system)
- [Generics](/schema/generics)
- [CLI Overview](/cli/overview)
