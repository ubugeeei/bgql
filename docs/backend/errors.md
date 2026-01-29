# Error Handling

Better GraphQL promotes type-safe error handling with errors as first-class types.

## Error Types

### Define Error Types in Schema

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
  value: Option<String>
}

type AuthenticationError implements Error {
  message: String
  code: String
}

type ForbiddenError implements Error {
  message: String
  code: String
  requiredPermission: String
}
```

### Use Unions for Results

```graphql
union UserResult = User | NotFoundError | AuthenticationError
union CreateUserResult = User | ValidationError

type Query {
  user(id: ID): UserResult
}

type Mutation {
  createUser(input: CreateUserInput): CreateUserResult
}
```

## Error Classes

Better GraphQL provides built-in error classes:

```typescript
import {
  BgqlError,
  AuthenticationError,
  ForbiddenError,
  NotFoundError,
  ValidationError,
  UserInputError,
} from '@bgql/server';
```

### Using Error Classes

```typescript
import { defineResolvers, NotFoundError, ValidationError } from '@bgql/server';

const resolvers = defineResolvers({
  Query: {
    user: async (_, { id }, { db }) => {
      const user = await db.users.findById(id);

      if (!user) {
        return {
          __typename: 'NotFoundError',
          message: `User with ID ${id} not found`,
          code: 'USER_NOT_FOUND',
          resourceType: 'User',
          resourceId: id,
        };
      }

      return { __typename: 'User', ...user };
    },
  },

  Mutation: {
    createUser: async (_, { input }, { db }) => {
      // Validate email uniqueness
      const existing = await db.users.findByEmail(input.email);
      if (existing) {
        return {
          __typename: 'ValidationError',
          message: 'Email already exists',
          code: 'EMAIL_EXISTS',
          field: 'email',
          value: input.email,
        };
      }

      const user = await db.users.create(input);
      return { __typename: 'User', ...user };
    },
  },
});
```

## Throwing Errors

For critical errors, you can throw:

```typescript
import { AuthenticationError, ForbiddenError } from '@bgql/server';

const resolvers = defineResolvers({
  Query: {
    adminDashboard: async (_, __, { user }) => {
      if (!user) {
        throw new AuthenticationError('Must be logged in');
      }

      if (user.role !== 'ADMIN') {
        throw new ForbiddenError('Admin access required');
      }

      return getAdminStats();
    },
  },
});
```

## Validation Errors

### Schema-Level Validation

Directives handle validation automatically:

```graphql
input CreateUserInput {
  name: String @minLength(1) @maxLength(100)
  email: String @email
  password: String @minLength(8)
}
```

Validation errors are returned as:

```json
{
  "__typename": "ValidationError",
  "message": "name must be at least 1 character",
  "code": "MIN_LENGTH",
  "field": "name"
}
```

### Custom Validation

```typescript
import { validateInput, ValidationError } from '@bgql/server';

const resolvers = defineResolvers({
  Mutation: {
    createUser: async (_, { input }, { db }) => {
      const errors = [];

      // Custom validation logic
      if (input.password !== input.confirmPassword) {
        errors.push({
          field: 'confirmPassword',
          message: 'Passwords do not match',
          code: 'PASSWORD_MISMATCH',
        });
      }

      if (await db.users.emailExists(input.email)) {
        errors.push({
          field: 'email',
          message: 'Email already registered',
          code: 'EMAIL_EXISTS',
        });
      }

      if (errors.length > 0) {
        return {
          __typename: 'ValidationErrors',
          errors,
        };
      }

      const user = await db.users.create(input);
      return { __typename: 'User', ...user };
    },
  },
});
```

## Multiple Errors

Return multiple validation errors at once:

```graphql
type ValidationErrors {
  errors: List<ValidationError>
}

union CreateUserResult = User | ValidationErrors
```

```typescript
const resolvers = defineResolvers({
  Mutation: {
    createUser: async (_, { input }) => {
      const errors = await validateUser(input);

      if (errors.length > 0) {
        return {
          __typename: 'ValidationErrors',
          errors: errors.map(e => ({
            ...e,
            __typename: 'ValidationError',
          })),
        };
      }

      // Create user...
    },
  },
});
```

## Client-Side Handling

### Type Guards

```typescript
import { isTypename, matchUnion } from '@bgql/client';

const result = await client.query(GetUser, { id: '1' });

// Using type guard
if (isTypename('User')(result.user)) {
  console.log(result.user.name);
} else if (isTypename('NotFoundError')(result.user)) {
  console.log(`Not found: ${result.user.resourceId}`);
}
```

### Pattern Matching

```typescript
import { matchUnion } from '@bgql/client';

const result = await client.mutate(CreateUser, { input });

matchUnion(result.createUser, {
  User: (user) => {
    toast.success(`Welcome, ${user.name}!`);
    router.push(`/users/${user.id}`);
  },
  ValidationError: (error) => {
    setFieldError(error.field, error.message);
  },
  ValidationErrors: (errors) => {
    errors.errors.forEach(e => setFieldError(e.field, e.message));
  },
});
```

### Exhaustive Handling

TypeScript ensures all cases are handled:

```typescript
import { assertNever } from '@bgql/client';

function handleResult(result: UserResult): string {
  switch (result.__typename) {
    case 'User':
      return `Hello, ${result.name}`;
    case 'NotFoundError':
      return `User not found: ${result.resourceId}`;
    case 'AuthenticationError':
      return 'Please log in';
    default:
      return assertNever(result);  // Compile error if case missing
  }
}
```

## Error Logging

### Server-Side Logging

```typescript
import { serve } from '@bgql/server';

serve({
  schema: './schema.bgql',
  resolvers,
  onError: (error, context) => {
    // Log to your error tracking service
    logger.error('GraphQL Error', {
      message: error.message,
      code: error.code,
      stack: error.stack,
      requestId: context.requestId,
      userId: context.user?.id,
    });

    // Report to error tracking
    Sentry.captureException(error, {
      extra: {
        requestId: context.requestId,
        operation: context.operationName,
      },
    });
  },
});
```

### Error Formatting

```typescript
serve({
  schema: './schema.bgql',
  resolvers,
  formatError: (error) => {
    // Don't expose internal errors to clients
    if (error.code === 'INTERNAL_ERROR') {
      return {
        message: 'An unexpected error occurred',
        code: 'INTERNAL_ERROR',
      };
    }

    return error;
  },
});
```

## Best Practices

### 1. Use Typed Errors

```graphql
# ✅ Good: Typed errors
union CreateUserResult = User | ValidationError | EmailExistsError

# ❌ Avoid: Generic error strings
type Mutation {
  createUser(input: CreateUserInput): User  # Throws on error
}
```

### 2. Be Specific

```graphql
# ✅ Good: Specific error types
type NotFoundError implements Error {
  message: String
  code: String
  resourceType: String
  resourceId: ID
}

# ❌ Avoid: Generic errors
type Error {
  message: String
}
```

### 3. Include Error Codes

```graphql
# ✅ Good: Machine-readable codes
type ValidationError {
  message: String       # "Email is invalid"
  code: String          # "INVALID_EMAIL"
  field: String         # "email"
}
```

### 4. Handle All Cases

```typescript
// ✅ Good: Handle all cases
matchUnion(result.createUser, {
  User: handleSuccess,
  ValidationError: handleValidation,
  EmailExistsError: handleEmailExists,
});

// ❌ Avoid: Ignoring error cases
if (result.createUser.__typename === 'User') {
  // What about errors?
}
```

### 5. Log Appropriately

```typescript
// ✅ Good: Log server errors, not user errors
onError: (error, context) => {
  if (error instanceof InternalError) {
    logger.error(error);
    Sentry.captureException(error);
  } else {
    logger.debug('User error', { code: error.code });
  }
}
```

## Next Steps

- [DataLoader](/backend/dataloader)
- [Testing](/backend/testing)
- [Context](/backend/context)
