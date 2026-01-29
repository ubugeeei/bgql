# Enums and Unions

Better GraphQL provides powerful enums and unions for modeling complex data.

## Enums

### Basic Enum

```graphql
enum UserRole {
  ADMIN
  MODERATOR
  USER
  GUEST
}

type User {
  id: ID
  name: String
  role: UserRole
}
```

### Enum with Documentation

```graphql
"""
User access level in the system.
"""
enum UserRole {
  """Full system access"""
  ADMIN

  """Can moderate content"""
  MODERATOR

  """Standard user"""
  USER

  """Limited read-only access"""
  GUEST
}
```

### Enum with Associated Data (Rust-style)

Better GraphQL supports Rust-style enums with data:

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

Usage in resolvers:

```typescript
// Creating enum variants
const result = { __variant: 'Ok', value: 'Success!' };
const error = { __variant: 'Err', code: 404, message: 'Not found' };

// Pattern matching
switch (result.__variant) {
  case 'Ok':
    console.log(result.value);
    break;
  case 'Err':
    console.log(`Error ${result.code}: ${result.message}`);
    break;
}
```

## Unions

### Basic Union

```graphql
type User {
  id: ID
  name: String
}

type NotFoundError {
  message: String
  resourceId: ID
}

type UnauthorizedError {
  message: String
}

union UserResult = User | NotFoundError | UnauthorizedError
```

### Union in Queries

```graphql
type Query {
  user(id: ID): UserResult
}

type Mutation {
  createUser(input: CreateUserInput): CreateUserResult
}
```

### Querying Unions

```graphql
query GetUser($id: ID!) {
  user(id: $id) {
    ... on User {
      id
      name
    }
    ... on NotFoundError {
      message
      resourceId
    }
    ... on UnauthorizedError {
      message
    }
  }
}
```

### Type-Safe Union Handling

```typescript
import { matchUnion } from '@bgql/client';

const result = await client.query(GetUser, { id: '1' });

const message = matchUnion(result.user, {
  User: (user) => `Hello, ${user.name}!`,
  NotFoundError: (err) => `Not found: ${err.resourceId}`,
  UnauthorizedError: (err) => `Access denied: ${err.message}`,
});
```

## Input Unions

Better GraphQL supports unions in input types:

```graphql
input union LoginMethod {
  EmailLogin { email: String, password: String }
  OAuthLogin { provider: String, token: String }
  PhoneLogin { phone: String, code: String }
}

type Mutation {
  login(method: LoginMethod): AuthResult
}
```

Usage:

```graphql
mutation LoginWithEmail {
  login(method: {
    EmailLogin: {
      email: "user@example.com"
      password: "secret"
    }
  }) {
    ... on AuthSuccess {
      token
    }
    ... on AuthError {
      message
    }
  }
}
```

## Input Enums

Rust-style enums for inputs:

```graphql
input enum SearchFilter {
  ByUser { userId: ID }
  ByTag { tag: String }
  ByDateRange { start: DateTime, end: DateTime }
  All
}

type Query {
  search(filter: SearchFilter): List<SearchResult>
}
```

## Result Pattern

A common pattern for error handling:

```graphql
interface Error {
  message: String
  code: String
}

type ValidationError implements Error {
  message: String
  code: String
  field: String
  rule: String
}

type NotFoundError implements Error {
  message: String
  code: String
  resourceType: String
  resourceId: ID
}

type User {
  id: ID
  name: String
}

union CreateUserResult = User | ValidationError
union GetUserResult = User | NotFoundError
```

### Handling Results

```typescript
import { isTypename, matchUnion } from '@bgql/client';

const result = await client.mutate(CreateUser, { input });

// Using type guard
if (isTypename('User')(result.createUser)) {
  console.log(`Created user: ${result.createUser.name}`);
} else {
  console.log(`Error: ${result.createUser.message}`);
}

// Using pattern matching
matchUnion(result.createUser, {
  User: (user) => {
    toast.success(`Created ${user.name}`);
    router.push(`/users/${user.id}`);
  },
  ValidationError: (err) => {
    setFieldError(err.field, err.message);
  },
});
```

## Generic Unions

Unions can be used with generic types:

```graphql
type Success<T> {
  data: T
}

type Failure<E extends Error> {
  error: E
}

# Note: Generic unions are expanded at use sites
type Query {
  # Expands to Success<User> | Failure<NotFoundError>
  user(id: ID): Success<User> | Failure<NotFoundError>
}
```

## TypeScript Generation

### Enum Generation

```graphql
enum Status {
  ACTIVE
  INACTIVE
  PENDING
}
```

```typescript
// Generated TypeScript
export const Status = {
  ACTIVE: 'ACTIVE',
  INACTIVE: 'INACTIVE',
  PENDING: 'PENDING',
} as const;

export type Status = typeof Status[keyof typeof Status];
```

### Union Generation

```graphql
union SearchResult = User | Post | Comment
```

```typescript
// Generated TypeScript
export type SearchResult =
  | ({ __typename: 'User' } & User)
  | ({ __typename: 'Post' } & Post)
  | ({ __typename: 'Comment' } & Comment);

// Type guards
export function isUser(value: SearchResult): value is User {
  return value.__typename === 'User';
}
```

## Best Practices

### 1. Use Unions for Polymorphism

```graphql
# ✅ Good: Clear polymorphism with __typename
union NotificationContent = Comment | Like | Follow | Mention

type Notification {
  id: ID
  createdAt: DateTime
  content: NotificationContent
}
```

### 2. Error Types Should Implement Error Interface

```graphql
# ✅ Good: Consistent error interface
interface Error {
  message: String
  code: String
}

type NotFoundError implements Error {
  message: String
  code: String
  resourceId: ID
}

type ValidationError implements Error {
  message: String
  code: String
  field: String
}
```

### 3. Use Enums for Fixed Sets

```graphql
# ✅ Good: Fixed set of values
enum OrderStatus {
  PENDING
  CONFIRMED
  SHIPPED
  DELIVERED
  CANCELLED
}

# ❌ Avoid: Using String for fixed values
type Order {
  status: String  # Could be anything!
}
```

### 4. Document Enum Values

```graphql
# ✅ Good: Documented values
enum Priority {
  """Requires immediate attention"""
  CRITICAL

  """Should be addressed soon"""
  HIGH

  """Normal priority"""
  MEDIUM

  """Can wait"""
  LOW
}
```

## Next Steps

- [Input Types](/schema/inputs)
- [Generics](/schema/generics)
- [Directives](/schema/directives)
