# Server API Reference

Complete API reference for `@bgql/server`.

## Installation

```bash
npm install @bgql/server
```

## Core Functions

### serve

Starts a production GraphQL server.

```typescript
import { serve } from '@bgql/server';

const server = await serve({
  schema: './schema.bgql',
  resolvers,
  port: 4000,
});
```

#### Options

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `schema` | `string` | - | Path to schema file(s) |
| `resolvers` | `Resolvers` | - | Resolver map |
| `port` | `number` | `4000` | Server port |
| `host` | `string` | `'0.0.0.0'` | Server host |
| `path` | `string` | `'/graphql'` | GraphQL endpoint path |
| `context` | `ContextFn` | - | Context factory function |
| `cors` | `CorsOptions \| boolean` | `true` | CORS configuration |
| `playground` | `boolean` | `false` | Enable GraphQL Playground |
| `introspection` | `boolean` | `true` | Enable introspection |
| `formatError` | `ErrorFormatter` | - | Custom error formatter |
| `onError` | `ErrorHandler` | - | Error callback |

#### Returns

```typescript
interface ServerInstance {
  readonly url: string;
  readonly port: number;
  close(): Promise<void>;
}
```

### devServer

Starts a development server with hot reload.

```typescript
import { devServer } from '@bgql/server';

await devServer({
  schema: './schema.bgql',
  resolvers: './src/resolvers',
  port: 4000,
  playground: true,
});
```

#### Additional Options

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `watch` | `boolean` | `true` | Watch for file changes |
| `playground` | `boolean` | `true` | Enable Playground |

### createTestClient

Creates a client for testing resolvers.

```typescript
import { createTestClient } from '@bgql/server';

const client = createTestClient({
  schema: './schema.bgql',
  resolvers,
  context: () => ({ user: mockUser }),
});

const result = await client.query({
  query: `query { user(id: "1") { name } }`,
});
```

#### Options

| Option | Type | Description |
|--------|------|-------------|
| `schema` | `string` | Path to schema file |
| `resolvers` | `Resolvers` | Resolver map |
| `context` | `ContextFn` | Context factory |
| `defaultContext` | `Context` | Default context object |

#### Methods

```typescript
interface TestClient {
  query<T>(options: QueryOptions): Promise<Result<T>>;
  mutate<T>(options: MutationOptions): Promise<Result<T>>;
  subscribe<T>(options: SubscriptionOptions): AsyncIterator<T>;
}
```

### defineResolvers

Creates type-safe resolvers.

```typescript
import { defineResolvers } from '@bgql/server';

interface Context {
  db: Database;
  user: User | null;
}

const resolvers = defineResolvers<Context>({
  Query: {
    user: async (_, { id }, ctx) => {
      return ctx.db.users.findById(id);
    },
  },
  Mutation: {
    createUser: async (_, { input }, ctx) => {
      return ctx.db.users.create(input);
    },
  },
});
```

### createResolver

Creates a single resolver with middleware.

```typescript
import { createResolver, withAuth } from '@bgql/server';

const myResolver = createResolver(
  async (parent, args, ctx) => {
    return ctx.db.data.find(args.id);
  },
  [withAuth()]  // Middleware chain
);
```

## Middleware

### withAuth

Requires authentication.

```typescript
import { withAuth } from '@bgql/server';

const resolvers = defineResolvers({
  Query: {
    me: withAuth(async (_, __, { user }) => user),
  },
});
```

#### Options

```typescript
withAuth(resolver, options?: {
  role?: string;
  permission?: string;
  message?: string;
})
```

### withValidation

Validates input.

```typescript
import { withValidation } from '@bgql/server';

const resolvers = defineResolvers({
  Mutation: {
    createUser: withValidation(
      async (_, { input }, ctx) => ctx.db.users.create(input),
      {
        input: {
          name: { minLength: 1, maxLength: 100 },
          email: { email: true },
        },
      }
    ),
  },
});
```

### withLogging

Adds logging.

```typescript
import { withLogging } from '@bgql/server';

const resolvers = defineResolvers({
  Mutation: {
    deleteUser: withLogging(
      async (_, { id }, ctx) => ctx.db.users.delete(id),
      { level: 'warn' }
    ),
  },
});
```

### withCaching

Adds response caching.

```typescript
import { withCaching } from '@bgql/server';

const resolvers = defineResolvers({
  Query: {
    config: withCaching(
      async () => loadConfig(),
      { ttl: 3600, scope: 'PUBLIC' }
    ),
  },
});
```

## Error Classes

### BgqlError

Base error class.

```typescript
import { BgqlError } from '@bgql/server';

throw new BgqlError('Something went wrong', 'ERROR_CODE');
```

### AuthenticationError

For authentication failures.

```typescript
import { AuthenticationError } from '@bgql/server';

if (!ctx.user) {
  throw new AuthenticationError('Must be logged in');
}
```

### ForbiddenError

For authorization failures.

```typescript
import { ForbiddenError } from '@bgql/server';

if (ctx.user.role !== 'ADMIN') {
  throw new ForbiddenError('Admin access required');
}
```

### NotFoundError

For missing resources.

```typescript
import { NotFoundError } from '@bgql/server';

const user = await db.users.findById(id);
if (!user) {
  throw new NotFoundError('User', id);
}
```

### ValidationError

For validation failures.

```typescript
import { ValidationError } from '@bgql/server';

if (!isValidEmail(input.email)) {
  throw new ValidationError('Invalid email', 'email');
}
```

## Types

### Resolver Types

```typescript
type ResolverFn<TParent, TArgs, TContext, TResult> = (
  parent: TParent,
  args: TArgs,
  context: TContext,
  info: GraphQLResolveInfo
) => TResult | Promise<TResult>;

type Resolvers<TContext = any> = {
  Query?: Record<string, ResolverFn<{}, any, TContext, any>>;
  Mutation?: Record<string, ResolverFn<{}, any, TContext, any>>;
  Subscription?: Record<string, SubscriptionResolver>;
  [typeName: string]: Record<string, ResolverFn<any, any, TContext, any>>;
};
```

### Context Types

```typescript
type ContextFn<TContext> = (
  req: Request
) => TContext | Promise<TContext>;

interface BaseContext {
  requestId: string;
  logger: Logger;
}
```

### Server Options

```typescript
interface ServeOptions<TContext = any> {
  schema: string | string[];
  resolvers: Resolvers<TContext>;
  port?: number;
  host?: string;
  path?: string;
  context?: ContextFn<TContext>;
  cors?: CorsOptions | boolean;
  playground?: boolean;
  introspection?: boolean;
  formatError?: (error: GraphQLError) => GraphQLFormattedError;
  onError?: (error: Error, context: TContext) => void;
}

interface CorsOptions {
  origin?: string | string[] | boolean;
  methods?: string[];
  allowedHeaders?: string[];
  credentials?: boolean;
  maxAge?: number;
}
```

## Utilities

### parseSchema

Parses a schema file.

```typescript
import { parseSchema } from '@bgql/server';

const schema = parseSchema('./schema.bgql');
```

### validateSchema

Validates a schema.

```typescript
import { validateSchema } from '@bgql/server';

const errors = validateSchema('./schema.bgql');
if (errors.length > 0) {
  console.error('Schema errors:', errors);
}
```

### formatSchema

Formats a schema file.

```typescript
import { formatSchema } from '@bgql/server';

const formatted = formatSchema('./schema.bgql');
await fs.writeFile('./schema.bgql', formatted);
```

## Subscriptions

### Setting Up Subscriptions

```typescript
import { serve, pubsub } from '@bgql/server';

const resolvers = defineResolvers({
  Subscription: {
    messageCreated: {
      subscribe: (_, { channelId }) => {
        return pubsub.subscribe(`channel:${channelId}`);
      },
      resolve: (payload) => payload,
    },
  },
  Mutation: {
    sendMessage: async (_, { input }, ctx) => {
      const message = await ctx.db.messages.create(input);
      pubsub.publish(`channel:${input.channelId}`, message);
      return message;
    },
  },
});
```

### PubSub API

```typescript
interface PubSub {
  publish<T>(channel: string, payload: T): void;
  subscribe<T>(channel: string): AsyncIterator<T>;
}
```

## Next Steps

- [Client API](/api/client)
- [Quick Start](/backend/quickstart)
- [Resolvers](/backend/resolvers)
