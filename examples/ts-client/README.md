# BGQL TypeScript Client Example

A type-safe Better GraphQL client using the `@bgql/client` SDK.

> **Note**: This example uses `@bgql/client` for proper Result-based error handling,
> request deduplication, caching, and retry logic. The SDK provides all the
> infrastructure; this example adds typed wrappers for the specific schema.

## Core Principles

### 1. Errors as Values (No Throws)

Traditional GraphQL clients throw exceptions for errors, making error handling unpredictable. BGQL treats errors as first-class values in the type system.

```typescript
// Traditional (avoid this)
try {
  const user = await client.getUser({ id: "1" });
} catch (e) {
  // What type is e? Network error? Business error? Unknown
}

// BGQL approach
const result = await client.getUser(UserId("1"));

// Type-safe exhaustive handling
switch (result.__typename) {
  case "User":
    console.log(result.name);
    break;
  case "NotFoundError":
    console.log(result.message, result.resourceId);
    break;
  case "UnauthorizedError":
    console.log(result.message);
    break;
}
```

### 2. Full Type Safety

The client provides complete type safety from schema to application code.

```typescript
// Branded types prevent ID mixing
type UserId = string & { readonly __brand: "UserId" };
type PostId = string & { readonly __brand: "PostId" };

// Compile error: can't pass PostId where UserId expected
const user = await client.getUser(postId);  // Type error!
```

### 3. Partial Promise for Streaming

Support for `@defer` and `@stream` through incremental delivery.

```typescript
const result = await client.getUserWithAnalytics(id);
// User fields available immediately
console.log(result.user.name);
// Analytics may arrive later via @defer
if (result.analytics) {
  console.log(result.analytics.totalPosts);
}
```

### 4. Native AbortController

Standard Web API for request cancellation.

```typescript
const controller = new AbortController();
setTimeout(() => controller.abort(), 5000); // Cancel after 5s

try {
  const result = await client.getUser(id, controller.signal);
} catch (e) {
  if (e.name === "AbortError") {
    console.log("Request cancelled");
  }
}
```

## Project Structure

```
ts-client/
├── src/
│   ├── main.ts        # Example usage
│   ├── client.ts      # BGQL client implementation
│   └── types.ts       # Generated types (in real app)
├── package.json
└── tsconfig.json
```

## Running the Examples

```bash
# First, start the server (in another terminal)
cd ../ts-server && npm run dev

# Then run the client examples
cd examples/ts-client
npm install
npm start
```

## API Reference

### Creating a Client

```typescript
import { createClient } from "./client.js";

const client = createClient({
  endpoint: "http://localhost:4000/graphql",
  headers: {
    "X-Custom-Header": "value",
  },
});
```

### Authentication

```typescript
// Login and get token
const result = await client.login({
  email: "alice@example.com",
  password: "password123",
});

if (result.__typename === "AuthPayload") {
  // Set token for subsequent requests
  client.setToken(result.token);
}

// Clear token
client.clearToken();
```

### Query Methods

```typescript
// Get current user (null if not authenticated)
const me = await client.me();

// Get user by ID (returns typed union)
const userResult = await client.getUser(UserId("user_1"));

// List users with pagination
const users = await client.listUsers({ first: 10, after: "cursor_5" });

// Get post by ID
const post = await client.getPost(PostId("post_1"));

// List posts with pagination
const posts = await client.listPosts({ first: 10 });
```

### Mutation Methods

```typescript
// Create user (returns User | ValidationError)
const createResult = await client.createUser({
  name: "New User",
  email: "new@example.com",
  password: "securepass123",
});

// Create post (returns Post | ValidationError | UnauthorizedError)
const postResult = await client.createPost({
  title: "My Post",
  content: "Hello World",
  status: "Draft",
});

// Login (returns AuthPayload | InvalidCredentialsError | ValidationError)
const loginResult = await client.login({
  email: "user@example.com",
  password: "password",
});
```

## Error Handling Patterns

### Exhaustive Switch

TypeScript ensures all union members are handled:

```typescript
const result = await client.getUser(id);

switch (result.__typename) {
  case "User":
    return renderUser(result);
  case "NotFoundError":
    return render404(result.resourceId);
  case "UnauthorizedError":
    return redirectToLogin();
  // TypeScript error if a case is missing
}
```

### Type Guards

```typescript
function isUser(result: UserResult): result is User {
  return result.__typename === "User";
}

const result = await client.getUser(id);
if (isUser(result)) {
  console.log(result.name); // TypeScript knows this is User
}
```

### Helper Functions

```typescript
// Pattern: unwrapOr
function unwrapUser(result: UserResult, fallback: User): User {
  return result.__typename === "User" ? result : fallback;
}

// Pattern: mapResult
function mapUserResult<T>(
  result: UserResult,
  handlers: {
    User: (user: User) => T;
    NotFoundError: (err: NotFoundError) => T;
    UnauthorizedError: (err: UnauthorizedError) => T;
  }
): T {
  return handlers[result.__typename](result as any);
}
```

## Sample Output

```
=====================================
  BGQL TypeScript Client Example
=====================================

--- Example 1: Get User ---

Found user: Alice Johnson (alice@example.com)
  Role: Admin
  Bio: Software engineer and blogger

--- Example 2: Get Non-existent User ---

Expected error: User with id "nonexistent" not found

--- Example 3: List Users ---

Total users: 3
Has next page: false

Users:
  - Alice Johnson (Admin)
  - Bob Smith (Moderator)
  - Carol Williams (User)

--- Example 4: Create User (Valid) ---

Created user: Test User (ID: user_4)

--- Example 5: Create User (Invalid) ---

Validation error (expected): name must be at least 3 characters
  Field: name
  Constraint: See message for details

--- Example 6: Login Flow ---

Login successful!
  User: Alice Johnson
  Token: token_user_1_17065...
  Expires: 2024-01-30T00:00:00.000Z
```
