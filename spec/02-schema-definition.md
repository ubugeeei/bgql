# Better GraphQL Specification - Schema Definition Language

## 1. Overview

Better GraphQL Schema Definition Language (SDL) is a declarative language for defining API types and operations.

## 2. Schema Definition

### 2.1 Root Operation Types

```graphql
schema {
  query: Query
  mutation: Mutation
  subscription: Subscription
}

type Query {
  users: List<User>
  user(id: ID): UserResult
}

type Mutation {
  createUser(input: CreateUserInput): CreateUserResult
  updateUser(id: ID, input: UpdateUserInput): UpdateUserResult
  deleteUser(id: ID): DeleteUserResult
}

type Subscription {
  userCreated: User
  userUpdated(id: ID): User
}
```

### 2.2 Schema-level Directives

```graphql
schema
  @cors(
    origins: ["https://app.example.com", "https://admin.example.com"],
    methods: [GET, POST, OPTIONS],
    credentials: true,
    maxAge: 86400
  )
  @rateLimit(requests: 1000, window: "1h")
{
  query: Query
  mutation: Mutation
}
```

## 3. Type Definitions

### 3.1 Object Types

```graphql
"""
Represents a user in the system
"""
type User implements Node & Timestamped {
  """Unique identifier"""
  id: ID

  """Display name"""
  name: String

  """Email address (visible only to authenticated users)"""
  email: String @requireAuth

  """Profile image URL"""
  avatarUrl: Option<String>

  """Creation timestamp"""
  createdAt: DateTime

  """Last update timestamp"""
  updatedAt: Option<DateTime>

  """User's posts"""
  posts(first: Int = 10, after: Option<String>): PostConnection
}
```

### 3.2 Interface Types

```graphql
interface Node {
  id: ID
}

interface Timestamped {
  createdAt: DateTime
  updatedAt: Option<DateTime>
}

interface Authored {
  author: User
}
```

### 3.3 Union Types

```graphql
union SearchResult = User | Post | Comment | Tag
```

### 3.4 Enum Types

```graphql
enum UserRole {
  ADMIN
  MODERATOR
  USER
  GUEST
}

enum SortDirection {
  ASC
  DESC
}
```

### 3.5 Input Types

```graphql
input CreateUserInput {
  name: String @minLength(1) @maxLength(100)
  email: String @email
  password: String @minLength(8) @pattern(regex: "^(?=.*[A-Za-z])(?=.*\\d).+$")
  role: UserRole = USER
}

input UpdateUserInput @patch(type: User) {
  name: Option<String> @minLength(1) @maxLength(100)
  email: Option<String> @email
  bio: Option<String> @maxLength(500)
}
```

### 3.6 Input Union Types

```graphql
input union PaymentMethod = CreditCardInput | BankTransferInput | CryptoInput

input CreditCardInput {
  cardNumber: String @pattern(regex: "^\\d{16}$")
  expiryMonth: Int @min(1) @max(12)
  expiryYear: Int @min(2024)
  cvv: String @pattern(regex: "^\\d{3,4}$")
}

input BankTransferInput {
  bankCode: String
  accountNumber: String
  accountName: String
}

input CryptoInput {
  walletAddress: String
  currency: CryptoCurrency
}
```

## 4. Error Type Definitions

### 4.1 Basic Pattern

In Better GraphQL, errors are defined as types and included in union return types.

```graphql
"""Base error interface"""
interface Error {
  message: String
  code: String
}

type NotFoundError implements Error {
  message: String
  code: String  # "NOT_FOUND"
  resourceType: String
  resourceId: ID
}

type ValidationError implements Error {
  message: String
  code: String  # "VALIDATION_ERROR"
  field: String
  constraint: String
}

type UnauthorizedError implements Error {
  message: String
  code: String  # "UNAUTHORIZED"
}

type ForbiddenError implements Error {
  message: String
  code: String  # "FORBIDDEN"
  requiredPermission: String
}
```

### 4.2 Operation-specific Errors

```graphql
type EmailAlreadyExistsError implements Error {
  message: String
  code: String  # "EMAIL_ALREADY_EXISTS"
  existingEmail: String
}

type WeakPasswordError implements Error {
  message: String
  code: String  # "WEAK_PASSWORD"
  requirements: List<String>
}

union CreateUserResult =
  | User
  | ValidationError
  | EmailAlreadyExistsError
  | WeakPasswordError

type Mutation {
  createUser(input: CreateUserInput): CreateUserResult
}
```

### 4.3 Result Pattern

```graphql
type UserPayload {
  user: User
}

union UserResult =
  | UserPayload
  | NotFoundError
  | UnauthorizedError

type Query {
  user(id: ID): UserResult
}
```

## 5. Server-side Fragments

### 5.1 Definition

Server-side Fragments are reusable field sets managed on the server.

```graphql
"""Basic user information"""
fragment UserBasic on User @server {
  id
  name
  avatarUrl
}

"""Detailed user information"""
fragment UserDetail on User @server {
  ...UserBasic
  email
  bio
  createdAt
}

"""Post preview"""
fragment PostPreview on Post @server {
  id
  title
  excerpt
  author {
    ...UserBasic
  }
  createdAt
}
```

### 5.2 Usage

Clients can use Server-side Fragments with the `@use` directive.

```graphql
query GetUser($id: ID) {
  user(id: $id) {
    ... on User @use(fragment: "UserDetail") {
      # UserDetail fragment fields are expanded
    }
  }
}
```

Or using shorthand syntax:

```graphql
query GetUser($id: ID) {
  user(id: $id) {
    ... on User {
      ...@UserDetail
    }
  }
}
```

### 5.3 Fragment Versioning

```graphql
fragment UserBasic on User @server @version("2024-01") {
  id
  name
  avatarUrl
}

fragment UserBasic on User @server @version("2023-06") @deprecated {
  id
  name
  avatar  # Old field name
}
```

## 6. Field Definitions

### 6.1 Arguments

```graphql
type Query {
  users(
    """Search query"""
    query: Option<String>,

    """Number of items to fetch"""
    first: Int = 10 @min(1) @max(100),

    """Cursor for pagination"""
    after: Option<String>,

    """Sort order"""
    orderBy: UserOrderBy = CREATED_AT_DESC,

    """Filter conditions"""
    filter: Option<UserFilter>
  ): UserConnection
}
```

### 6.2 Default Values

```graphql
input UserFilter {
  role: UserRole = User
  isActive: Boolean = true
  createdAfter: Option<DateTime>
}
```

## 7. Documentation

### 7.1 Description Strings

Strings enclosed in triple double quotes are documentation comments.

```graphql
"""
Represents a user account.
Contains information about authenticated users.

## Fields

- `id`: Unique identifier
- `name`: Display name
- `email`: Email address (visible only to authenticated users)
"""
type User {
  """Unique identifier (UUID v4)"""
  id: ID

  """
  Display name.
  Must be between 1-100 characters.
  """
  name: String
}
```

### 7.2 Comments

```graphql
# This is a comment (not included in documentation)
type User {
  id: ID  # Inline comment
}
```

## 8. Extend

### 8.1 Type Extension

```graphql
# Base definition
type User {
  id: ID
  name: String
}

# Extension
extend type User {
  email: String
  posts: List<Post>
}
```

### 8.2 Schema Extension

```graphql
extend schema {
  subscription: Subscription
}

extend type Query {
  searchUsers(query: String): List<User>
}
```

## 9. Reserved Words

The following identifiers are reserved and cannot be used as type or field names:

- `query`, `mutation`, `subscription`
- `type`, `interface`, `union`, `enum`, `input`, `scalar`
- `fragment`, `on`, `extend`, `schema`
- `directive`
- `true`, `false`, `null`
- `__typename`, `__schema`, `__type` (for introspection)

## 10. Module System

Better GraphQL uses a Rust-inspired module system with `mod`, `use`, and `pub` keywords for organizing schemas across files.

### 10.1 Visibility

By default, all definitions are private to their module. Use `pub` to make them visible to other modules.

```graphql
# Private type (only visible within same module)
type InternalConfig {
  secret: String
}

# Public type (visible to other modules)
pub type User {
  id: ID
  name: String
}

# Public enum
pub enum UserRole {
  Admin
  User
  Guest
}
```

### 10.2 Module Declaration

Use `mod` to declare submodules. Modules can be external (in separate files) or inline.

```graphql
# External module declaration (loads from users.bgql or users/mod.bgql)
mod users;

# External module declaration with path
mod auth;
mod posts;

# Inline module
mod helpers {
  pub type PageInfo {
    hasNextPage: Boolean
    hasPreviousPage: Boolean
    startCursor: Option<String>
    endCursor: Option<String>
  }
}

# Public module (re-exports are visible to parent's importers)
pub mod common;
```

### 10.3 Import Syntax (use)

Use `use` to import types from other modules.

```graphql
# Import specific items
use::users::{User, UserInput}
use::posts::Post
use::common::PageInfo

# Import with alias
use::external::User as ExternalUser

# Glob import (import all public items)
use::common::*

# Re-export (make imported items public)
pub use::users::User
```

### 10.4 Module Path Resolution

Modules are resolved relative to the current file:

- `mod users;` → `./users.bgql` or `./users/mod.bgql`
- `use::users::User` → Import `User` from the `users` module
- `use::users::types::User` → Import `User` from `users/types` submodule

### 10.5 Recommended Structure

```
schema/
├── mod.bgql              # Root module
├── users/
│   ├── mod.bgql          # User module root
│   ├── types.bgql        # User types
│   └── inputs.bgql       # User inputs
├── posts/
│   ├── mod.bgql
│   └── types.bgql
├── common/
│   ├── mod.bgql
│   ├── errors.bgql       # Shared error types
│   └── pagination.bgql   # Pagination types
└── directives/
    └── mod.bgql          # Custom directives
```

### 10.6 Root Module Example

```graphql
# mod.bgql (root)

# Declare submodules
mod users;
mod posts;
mod common;

# Import from submodules
use::users::{User, UserResult}
use::posts::{Post, PostConnection}
use::common::{PageInfo, NotFoundError, ValidationError}

# Re-export commonly used types
pub use::common::PageInfo

schema {
  query: Query
  mutation: Mutation
}

type Query {
  user(id: ID): UserResult
  posts(first: Int, after: Option<String>): PostConnection
}

type Mutation {
  createUser(input: CreateUserInput): CreateUserResult
}
```

### 10.7 Submodule Example

```graphql
# users/mod.bgql

# Declare child modules
mod types;
mod inputs;

# Re-export public items
pub use::types::{User, UserPayload}
pub use::inputs::{CreateUserInput, UpdateUserInput}

# Import from sibling modules
use::common::{NotFoundError, ValidationError}

# Define types specific to this module
pub union UserResult = UserPayload | NotFoundError

pub union CreateUserResult =
  | UserPayload
  | ValidationError
  | EmailAlreadyExistsError

type EmailAlreadyExistsError implements Error {
  message: String
  code: String
  email: String
}
```

### 10.8 Types Module Example

```graphql
# users/types.bgql

use::common::Timestamped

"""
Represents a user in the system.
"""
pub type User implements Node & Timestamped {
  id: ID
  name: String
  email: String @requireAuth
  avatarUrl: Option<String>
  createdAt: DateTime
  updatedAt: Option<DateTime>
}

pub type UserPayload {
  user: User
}
```

### 10.9 Module Visibility Rules

1. **Private by default**: All definitions are private unless marked `pub`
2. **Public visibility**: `pub` makes a definition visible to parent modules
3. **Re-exports**: `pub use` makes imported items visible to importers of this module
4. **Module hierarchy**: A module can only access:
   - Its own definitions
   - Public definitions from declared submodules
   - Items imported via `use`
