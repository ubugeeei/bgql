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

## 10. File Organization

### 10.1 Recommended Structure

```
schema/
├── schema.bgql           # Root definition
├── types/
│   ├── user.bgql
│   ├── post.bgql
│   └── comment.bgql
├── inputs/
│   ├── user.bgql
│   └── post.bgql
├── errors/
│   └── common.bgql
├── fragments/
│   └── server-fragments.bgql
└── directives/
    └── custom.bgql
```

### 10.2 Import Syntax

```graphql
# schema.bgql
import { User, UserResult } from "./types/user.bgql"
import { CreateUserInput } from "./inputs/user.bgql"
import { NotFoundError, ValidationError } from "./errors/common.bgql"
```
