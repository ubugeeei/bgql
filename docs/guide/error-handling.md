# Error Handling

Better GraphQL provides type-safe error handling through union types and the Result pattern.

## Philosophy

Traditional GraphQL uses nullable fields and a global `errors` array, making error handling fragile. Better GraphQL:

1. **Explicit errors** - Error types are part of the schema
2. **Type-safe** - TypeScript knows all possible error states
3. **Localized** - Errors are returned where they occur
4. **Exhaustive** - Pattern matching ensures all cases handled

## Error as Union Members

### Defining Error Types

```graphql
# Schema
type NotFoundError {
  message: String
  resourceType: String
  resourceId: ID
}

type ValidationError {
  message: String
  field: String
  code: String
}

type AuthError {
  message: String
  requiredRole: Option<String>
}

# Use in queries
union UserResult = User | NotFoundError | AuthError

type Query {
  user(id: ID): UserResult
}
```

### Querying with Errors

```graphql
query GetUser($id: ID!) {
  user(id: $id) {
    ... on User {
      id
      name
      email
    }
    ... on NotFoundError {
      message
      resourceId
    }
    ... on AuthError {
      message
    }
  }
}
```

### Handling in TypeScript

```typescript
import { matchUnion, isTypename } from '@bgql/client';

const result = await client.query(GetUserDocument, { id: '1' });

if (result.ok) {
  matchUnion(result.value.user, {
    User: (user) => {
      console.log(`Hello, ${user.name}!`);
    },
    NotFoundError: (error) => {
      console.log(`User ${error.resourceId} not found`);
    },
    AuthError: (error) => {
      console.log(`Authentication required: ${error.message}`);
    },
  });
}
```

## Error Interfaces

### Common Error Interface

```graphql
# Define a common error interface
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
  constraint: String
}

type RateLimitError implements Error {
  message: String
  code: String
  retryAfter: Int
}
```

### Handling Any Error

```typescript
function handleError(error: Error) {
  // Common handling for all error types
  console.error(`Error [${error.code}]: ${error.message}`);

  // Specific handling
  if (isTypename('RateLimitError')(error)) {
    setTimeout(() => retry(), error.retryAfter * 1000);
  }
}
```

## Result Pattern

### Network vs Domain Errors

Better GraphQL distinguishes between:

1. **Network errors** - Connection failures, timeouts
2. **Domain errors** - Business logic errors (part of schema)

```typescript
const result = await client.query(GetUser, { id: '1' });

// Check for network errors first
if (!result.ok) {
  // Network-level error
  switch (result.error.type) {
    case 'network':
      showOfflineMessage();
      break;
    case 'timeout':
      showRetryButton();
      break;
  }
  return;
}

// Now handle domain errors (part of the response)
matchUnion(result.value.user, {
  User: handleUser,
  NotFoundError: handleNotFound,
  AuthError: handleAuth,
});
```

### Result Helpers

```typescript
import { isOk, isErr, unwrap, unwrapOr, match } from '@bgql/client';

// Type guards
if (isOk(result)) {
  // result.value is available
}

if (isErr(result)) {
  // result.error is available
}

// Unwrap (throws on error)
const data = unwrap(result);

// Unwrap with default
const data = unwrapOr(result, { user: null });

// Pattern matching
const message = match(result, {
  ok: (data) => `Found user: ${data.user.name}`,
  err: (error) => `Failed: ${error.message}`,
});
```

## Mutation Errors

### Validation Errors

```graphql
type ValidationErrors {
  errors: List<ValidationError>
}

union CreateUserResult = User | ValidationErrors

type Mutation {
  createUser(input: CreateUserInput): CreateUserResult
}
```

```typescript
const result = await client.mutate(CreateUserDocument, {
  input: { name: '', email: 'invalid' },
});

if (result.ok) {
  matchUnion(result.value.createUser, {
    User: (user) => {
      toast.success('User created!');
      router.push(`/users/${user.id}`);
    },
    ValidationErrors: ({ errors }) => {
      errors.forEach(error => {
        form.setError(error.field, { message: error.message });
      });
    },
  });
}
```

### Multiple Error Types

```graphql
union UpdateUserResult =
  | User
  | NotFoundError
  | ValidationError
  | PermissionError

type Mutation {
  updateUser(id: ID, input: UpdateUserInput): UpdateUserResult
}
```

```typescript
matchUnion(result.value.updateUser, {
  User: (user) => {
    toast.success('Updated!');
  },
  NotFoundError: (error) => {
    toast.error('User not found');
    router.push('/users');
  },
  ValidationError: (error) => {
    form.setError(error.field, { message: error.message });
  },
  PermissionError: (error) => {
    toast.error('You do not have permission to edit this user');
  },
});
```

## Backend Error Handling

### Resolver Implementation

```typescript
import { ok, err, NotFoundError, ValidationError } from '@bgql/server';

const resolvers = {
  Query: {
    user: async (_, { id }, ctx) => {
      const user = await ctx.db.users.findById(id);

      if (!user) {
        return {
          __typename: 'NotFoundError',
          message: 'User not found',
          resourceType: 'User',
          resourceId: id,
        };
      }

      return { __typename: 'User', ...user };
    },
  },

  Mutation: {
    createUser: async (_, { input }, ctx) => {
      // Validate input
      const errors = validateCreateUser(input);
      if (errors.length > 0) {
        return {
          __typename: 'ValidationErrors',
          errors,
        };
      }

      // Check for existing email
      const existing = await ctx.db.users.findByEmail(input.email);
      if (existing) {
        return {
          __typename: 'ValidationError',
          message: 'Email already exists',
          field: 'email',
          code: 'EMAIL_EXISTS',
        };
      }

      // Create user
      const user = await ctx.db.users.create(input);
      return { __typename: 'User', ...user };
    },
  },
};
```

### Error Factories

```typescript
// errors.ts
export const errors = {
  notFound: (resourceType: string, resourceId: string) => ({
    __typename: 'NotFoundError' as const,
    message: `${resourceType} not found`,
    resourceType,
    resourceId,
    code: 'NOT_FOUND',
  }),

  validation: (field: string, message: string, code?: string) => ({
    __typename: 'ValidationError' as const,
    message,
    field,
    code: code ?? 'VALIDATION_FAILED',
  }),

  auth: (message: string, requiredRole?: string) => ({
    __typename: 'AuthError' as const,
    message,
    requiredRole,
    code: 'UNAUTHORIZED',
  }),

  permission: (message: string, action: string) => ({
    __typename: 'PermissionError' as const,
    message,
    action,
    code: 'FORBIDDEN',
  }),
};

// Usage in resolvers
return errors.notFound('User', id);
return errors.validation('email', 'Invalid email format');
```

## Global Error Handling

### Error Middleware

```typescript
import { createClient, errorMiddleware } from '@bgql/client';

const client = createClient('http://localhost:4000/graphql')
  .use(errorMiddleware({
    onNetworkError: (error) => {
      console.error('Network error:', error);
      showOfflineNotification();
    },
    onGraphQLError: (error) => {
      if (error.extensions?.code === 'UNAUTHENTICATED') {
        redirectToLogin();
      }
    },
  }));
```

### Error Boundary (React)

```tsx
import { ErrorBoundary } from '@bgql/client/react';

function App() {
  return (
    <ErrorBoundary
      fallback={({ error, resetError }) => (
        <div>
          <h1>Something went wrong</h1>
          <p>{error.message}</p>
          <button onClick={resetError}>Try again</button>
        </div>
      )}
    >
      <UserProfile userId="1" />
    </ErrorBoundary>
  );
}
```

## Exhaustive Handling

### assertNever Pattern

```typescript
import { assertNever } from '@bgql/client';

function handleResult(result: UserResult): string {
  switch (result.__typename) {
    case 'User':
      return result.name;
    case 'NotFoundError':
      return 'Not found';
    case 'AuthError':
      return 'Unauthorized';
    default:
      // TypeScript error if case is missing
      return assertNever(result);
  }
}
```

### matchUnion Exhaustiveness

```typescript
// TypeScript error if any case is missing
matchUnion(result.value.user, {
  User: handleUser,
  NotFoundError: handleNotFound,
  // Missing AuthError causes compile error
});
```

## Best Practices

### 1. Use Union Types for Expected Errors

```graphql
# Good: Error is part of the type
union UserResult = User | NotFoundError

type Query {
  user(id: ID): UserResult
}
```

### 2. Keep Error Types Specific

```graphql
# Good: Specific error types
type EmailExistsError {
  message: String
  existingEmail: String
}

type InvalidEmailError {
  message: String
  providedEmail: String
}

# Avoid: Generic error
type Error {
  message: String
}
```

### 3. Include Actionable Information

```graphql
# Good: Error includes retry information
type RateLimitError {
  message: String
  retryAfter: Int      # Seconds until retry allowed
  limit: Int           # Request limit
  remaining: Int       # Remaining requests
}
```

### 4. Handle All Cases

```typescript
// Always handle all possible outcomes
matchUnion(result.value.user, {
  User: handleSuccess,
  NotFoundError: handleNotFound,
  AuthError: handleAuth,
});
```

## Next Steps

- [Type System](/guide/type-system)
- [Backend Errors](/backend/errors)
- [Frontend Errors](/frontend/errors)
