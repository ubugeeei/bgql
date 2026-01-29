# Testing

Better GraphQL provides comprehensive testing utilities for your resolvers and GraphQL operations.

## Test Client

### Creating a Test Client

```typescript
import { createTestClient } from '@bgql/server';
import { resolvers } from './resolvers';

const client = createTestClient({
  schema: './schema.bgql',
  resolvers,
  context: () => ({
    db: mockDatabase,
    user: mockUser,
  }),
});
```

### Basic Query Testing

```typescript
import { describe, it, expect } from 'vitest';

describe('User queries', () => {
  it('should fetch a user by ID', async () => {
    const result = await client.query({
      query: `
        query GetUser($id: ID!) {
          user(id: $id) {
            ... on User {
              id
              name
              email
            }
            ... on NotFoundError {
              message
            }
          }
        }
      `,
      variables: { id: '1' },
    });

    expect(result.ok).toBe(true);
    expect(result.value.user.__typename).toBe('User');
    expect(result.value.user.name).toBe('John Doe');
  });
});
```

### Mutation Testing

```typescript
describe('User mutations', () => {
  it('should create a user', async () => {
    const result = await client.mutate({
      mutation: `
        mutation CreateUser($input: CreateUserInput!) {
          createUser(input: $input) {
            ... on User {
              id
              name
              email
            }
            ... on ValidationError {
              field
              message
            }
          }
        }
      `,
      variables: {
        input: {
          name: 'Jane Doe',
          email: 'jane@example.com',
        },
      },
    });

    expect(result.ok).toBe(true);
    expect(result.value.createUser.__typename).toBe('User');
    expect(result.value.createUser.name).toBe('Jane Doe');
  });

  it('should return validation error for invalid email', async () => {
    const result = await client.mutate({
      mutation: `
        mutation CreateUser($input: CreateUserInput!) {
          createUser(input: $input) {
            ... on User { id }
            ... on ValidationError {
              field
              message
            }
          }
        }
      `,
      variables: {
        input: {
          name: 'Jane',
          email: 'invalid-email',
        },
      },
    });

    expect(result.ok).toBe(true);
    expect(result.value.createUser.__typename).toBe('ValidationError');
    expect(result.value.createUser.field).toBe('email');
  });
});
```

## Mocking

### Mock Database

```typescript
import { vi } from 'vitest';

const mockDatabase = {
  users: {
    findById: vi.fn(),
    findByEmail: vi.fn(),
    create: vi.fn(),
    update: vi.fn(),
    delete: vi.fn(),
  },
  posts: {
    findById: vi.fn(),
    findByAuthor: vi.fn(),
    create: vi.fn(),
  },
};

beforeEach(() => {
  vi.clearAllMocks();
});

it('should call database with correct ID', async () => {
  mockDatabase.users.findById.mockResolvedValue({
    id: '1',
    name: 'John',
    email: 'john@example.com',
  });

  await client.query({
    query: `query { user(id: "1") { ... on User { id } } }`,
  });

  expect(mockDatabase.users.findById).toHaveBeenCalledWith('1');
});
```

### Mock Context

```typescript
function createMockContext(overrides = {}) {
  return {
    db: mockDatabase,
    user: null,
    requestId: 'test-request-id',
    logger: {
      info: vi.fn(),
      error: vi.fn(),
      warn: vi.fn(),
    },
    ...overrides,
  };
}

// Test with authenticated user
const authenticatedClient = createTestClient({
  schema: './schema.bgql',
  resolvers,
  context: () => createMockContext({
    user: { id: '1', role: 'ADMIN' },
  }),
});

// Test with unauthenticated user
const anonymousClient = createTestClient({
  schema: './schema.bgql',
  resolvers,
  context: () => createMockContext({ user: null }),
});
```

## Testing Authentication

```typescript
describe('Authentication', () => {
  it('should return auth error for protected queries', async () => {
    const result = await anonymousClient.query({
      query: `query { me { id name } }`,
    });

    expect(result.ok).toBe(false);
    expect(result.error.message).toContain('authenticated');
  });

  it('should allow access for authenticated users', async () => {
    mockDatabase.users.findById.mockResolvedValue({
      id: '1',
      name: 'John',
    });

    const result = await authenticatedClient.query({
      query: `query { me { id name } }`,
    });

    expect(result.ok).toBe(true);
    expect(result.value.me.name).toBe('John');
  });
});
```

## Testing Authorization

```typescript
describe('Authorization', () => {
  const userClient = createTestClient({
    schema: './schema.bgql',
    resolvers,
    context: () => createMockContext({
      user: { id: '2', role: 'USER' },
    }),
  });

  const adminClient = createTestClient({
    schema: './schema.bgql',
    resolvers,
    context: () => createMockContext({
      user: { id: '1', role: 'ADMIN' },
    }),
  });

  it('should deny admin routes to regular users', async () => {
    const result = await userClient.query({
      query: `query { adminDashboard { userCount } }`,
    });

    expect(result.ok).toBe(false);
    expect(result.error.message).toContain('Admin');
  });

  it('should allow admin routes to admins', async () => {
    const result = await adminClient.query({
      query: `query { adminDashboard { userCount } }`,
    });

    expect(result.ok).toBe(true);
  });
});
```

## Testing DataLoader

```typescript
import DataLoader from 'dataloader';

describe('DataLoader', () => {
  it('should batch user lookups', async () => {
    const batchFn = vi.fn(async (ids: readonly string[]) => {
      return ids.map(id => ({ id, name: `User ${id}` }));
    });

    const client = createTestClient({
      schema: './schema.bgql',
      resolvers,
      context: () => ({
        db: mockDatabase,
        loaders: {
          users: new DataLoader(batchFn),
        },
      }),
    });

    // Query that triggers multiple user lookups
    await client.query({
      query: `
        query {
          posts {
            author { id name }
          }
        }
      `,
    });

    // Should batch all user IDs into single call
    expect(batchFn).toHaveBeenCalledTimes(1);
    expect(batchFn).toHaveBeenCalledWith(
      expect.arrayContaining(['1', '2', '3'])
    );
  });
});
```

## Snapshot Testing

```typescript
import { expect, it } from 'vitest';

it('should match user query snapshot', async () => {
  mockDatabase.users.findById.mockResolvedValue({
    id: '1',
    name: 'John Doe',
    email: 'john@example.com',
    createdAt: '2024-01-01T00:00:00Z',
  });

  const result = await client.query({
    query: `
      query GetUser($id: ID!) {
        user(id: $id) {
          ... on User {
            id
            name
            email
            createdAt
          }
        }
      }
    `,
    variables: { id: '1' },
  });

  expect(result).toMatchSnapshot();
});
```

## Integration Testing

```typescript
import { serve } from '@bgql/server';
import { createClient } from '@bgql/client';

describe('Integration', () => {
  let server: Awaited<ReturnType<typeof serve>>;
  let client: ReturnType<typeof createClient>;

  beforeAll(async () => {
    server = await serve({
      schema: './schema.bgql',
      resolvers,
      port: 0,  // Random available port
    });

    client = createClient(server.url);
  });

  afterAll(async () => {
    await server.close();
  });

  it('should handle real HTTP requests', async () => {
    const result = await client.query(`
      query {
        users(first: 10) {
          edges {
            node { id name }
          }
        }
      }
    `);

    expect(result.ok).toBe(true);
  });
});
```

## Testing Subscriptions

```typescript
describe('Subscriptions', () => {
  it('should receive messages', async () => {
    const messages: any[] = [];

    const subscription = client.subscribe({
      query: `
        subscription OnMessage($channelId: ID!) {
          messageCreated(channelId: $channelId) {
            id
            content
          }
        }
      `,
      variables: { channelId: 'channel-1' },
    });

    // Collect messages
    const unsubscribe = subscription.subscribe({
      next: (data) => messages.push(data),
    });

    // Trigger a message
    await client.mutate({
      mutation: `
        mutation SendMessage($input: SendMessageInput!) {
          sendMessage(input: $input) { id }
        }
      `,
      variables: {
        input: { channelId: 'channel-1', content: 'Hello!' },
      },
    });

    // Wait for message to arrive
    await new Promise(resolve => setTimeout(resolve, 100));

    expect(messages).toHaveLength(1);
    expect(messages[0].messageCreated.content).toBe('Hello!');

    unsubscribe();
  });
});
```

## Test Utilities

### Custom Matchers

```typescript
// vitest.setup.ts
expect.extend({
  toBeGraphQLError(received, expectedCode) {
    const pass = received.ok === false &&
                 received.error.code === expectedCode;

    return {
      pass,
      message: () => pass
        ? `Expected not to be GraphQL error with code ${expectedCode}`
        : `Expected GraphQL error with code ${expectedCode}, got ${received.error?.code}`,
    };
  },

  toBeUser(received) {
    const pass = received.__typename === 'User' &&
                 typeof received.id === 'string' &&
                 typeof received.name === 'string';

    return {
      pass,
      message: () => pass
        ? 'Expected not to be a User'
        : 'Expected to be a User with id and name',
    };
  },
});

// Usage
expect(result).toBeGraphQLError('NOT_FOUND');
expect(result.value.user).toBeUser();
```

### Factory Functions

```typescript
// test/factories.ts
export function createUser(overrides = {}) {
  return {
    id: 'user-1',
    name: 'Test User',
    email: 'test@example.com',
    role: 'USER',
    createdAt: new Date().toISOString(),
    ...overrides,
  };
}

export function createPost(overrides = {}) {
  return {
    id: 'post-1',
    title: 'Test Post',
    content: 'Test content',
    authorId: 'user-1',
    createdAt: new Date().toISOString(),
    ...overrides,
  };
}

// Usage
mockDatabase.users.findById.mockResolvedValue(
  createUser({ name: 'John Doe' })
);
```

## Best Practices

### 1. Isolate Tests

```typescript
// ✅ Good: Fresh mock state for each test
beforeEach(() => {
  vi.clearAllMocks();
});

// ✅ Good: Isolated test client
const client = createTestClient({
  context: () => createMockContext(),
});
```

### 2. Test Error Cases

```typescript
// ✅ Good: Test both success and error paths
it('should return user', async () => { ... });
it('should return NotFoundError', async () => { ... });
it('should return ValidationError', async () => { ... });
```

### 3. Use Type-Safe Queries

```typescript
// ✅ Good: Use generated types
import { GetUserDocument } from './generated/graphql';

const result = await client.queryTyped(GetUserDocument, { id: '1' });
// result.value.user is fully typed
```

## Next Steps

- [Error Handling](/backend/errors)
- [DataLoader](/backend/dataloader)
- [Context](/backend/context)
