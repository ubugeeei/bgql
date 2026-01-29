# BGQL TypeScript Server Example

A complete TypeScript GraphQL server demonstrating BGQL's schema-first development with layered architecture.

## Features

- **Schema-First Development**: Schema defined in `.bgql` files with module system
- **Layered Architecture**: Domain-Driven Design with clear separation of concerns
- **Railway-Oriented Programming**: Result types for predictable error handling
- **DataLoader Integration**: Automatic N+1 query prevention
- **Typed Errors**: Union types for typed error responses
- **Bun Runtime**: Fast TypeScript execution with bun
- **Type Checking**: Uses `tsgo` (@typescript/native-preview) for type checking

## Project Structure

```
ts-server/
├── schema/                    # BGQL schema files
│   ├── mod.bgql              # Root module with exports
│   ├── types.bgql            # Domain types (User, Post, Comment)
│   ├── errors.bgql           # Typed error types
│   ├── inputs.bgql           # Input types with validation
│   ├── queries.bgql          # Query operations
│   └── mutations.bgql        # Mutation operations
├── src/
│   ├── domain/               # Domain Layer
│   │   ├── entities.ts       # Pure domain entities
│   │   ├── errors.ts         # Domain error classes
│   │   └── index.ts
│   ├── infrastructure/       # Infrastructure Layer
│   │   ├── repositories.ts   # Data access abstraction
│   │   ├── loaders.ts        # DataLoader factory
│   │   └── index.ts
│   ├── application/          # Application Layer
│   │   ├── user-use-cases.ts # User business logic
│   │   ├── post-use-cases.ts # Post business logic
│   │   └── index.ts
│   ├── presentation/         # Presentation Layer
│   │   ├── context.ts        # Request context setup
│   │   ├── resolvers.ts      # GraphQL resolvers
│   │   └── index.ts
│   └── server.ts             # Server entry point
├── package.json
└── tsconfig.json
```

## Architecture

### Domain Layer (`src/domain/`)

Pure business entities and errors without infrastructure concerns.

```typescript
// Branded types for type safety
type UserId = string & { readonly __brand: "UserId" };

// Domain entities
interface User {
  readonly id: UserId;
  readonly name: string;
  readonly email: Email;
  // ...
}

// Domain errors with GraphQL conversion
class UserNotFoundError extends DomainError {
  toGraphQL() { /* ... */ }
}
```

### Infrastructure Layer (`src/infrastructure/`)

Data access and external service integration.

```typescript
// Repository interface (returns Result types)
interface UserRepository {
  findById(id: UserId): Promise<Result<User, UserNotFoundError>>;
  create(data: CreateUserData): Promise<Result<User, UniqueConstraintError>>;
}

// DataLoader for N+1 prevention
const loaders = createLoaders(userRepo, postRepo, commentRepo);
```

### Application Layer (`src/application/`)

Business logic orchestration with use cases.

```typescript
// Query use case
class UserQueryService {
  async getUser(query: GetUserQuery): Promise<Result<User, UserNotFoundError>> {
    return this.userRepo.findById(query.id);
  }
}

// Command use case with validation
class UserCommandService {
  async createUser(command: CreateUserCommand): Promise<Result<User, ValidationError>> {
    // Validate, then create
  }
}
```

### Presentation Layer (`src/presentation/`)

GraphQL resolvers and request context.

```typescript
// Context with all dependencies
interface Context extends BaseContext {
  readonly currentUser: User | null;
  readonly loaders: Loaders;
  readonly services: Services;
}

// Resolvers use services
const resolvers = {
  Query: {
    user: async (_, args, ctx) => {
      const result = await ctx.services.userQuery.getUser({ id: args.id });
      return resultToUnion(result, userToGraphQL);
    }
  }
};
```

## Getting Started

### Prerequisites

- [Bun](https://bun.sh/) v1.0+
- [bgql CLI](../../) installed (`cargo install bgql`)
- [tsgo](https://www.npmjs.com/package/@typescript/native-preview) (optional, for type checking)

### Installation

```bash
cd examples/ts-server
bun install
```

### Development

```bash
# Compile schema and start with hot reload
bun run dev

# Or step by step:
bun run schema    # Compile .bgql to GraphQL SDL
bun run codegen   # Generate TypeScript types
bun run dev       # Start server
```

### Production Build

```bash
# Build for production (schema inlined via macros)
bun run build

# Start production server
bun run start
```

### Type Check

```bash
bun run typecheck
```

## Example Queries

### Get User (with typed error union)

```graphql
query GetUser($id: UserId!) {
  user(id: $id) {
    ... on User {
      id
      name
      email
      posts {
        title
      }
    }
    ... on NotFoundError {
      message
      resourceId
    }
  }
}
```

### Create User (with validation)

```graphql
mutation CreateUser($input: CreateUserInput!) {
  createUser(input: $input) {
    ... on User {
      id
      name
    }
    ... on ValidationError {
      field
      message
      constraint
    }
  }
}
```

### List Users with Posts

```graphql
query ListUsers {
  users {
    id
    name
    role
    posts {
      id
      title
      status
    }
    analytics {
      totalPosts
      totalComments
    }
  }
}
```

## Schema Features

The schema demonstrates BGQL-specific features:

```graphql
# Module system
mod types
mod errors
pub use types::*

# Opaque types (nominal typing)
opaque UserId = ID

# Generics
type Connection<T extends Node> {
  edges: List<Edge<T>>
  pageInfo: PageInfo
}

# Input unions
input union LoginCredentials = EmailCredentials | OAuthCredentials

# Validation directives
input CreateUserInput {
  name: String @minLength(2) @maxLength(100) @trim
  email: String @email @lowercase
}

# Typed error unions
union UserResult = User | NotFoundError | UnauthorizedError
```

## Railway-Oriented Programming

All operations return `Result<T, E>` types for predictable error handling:

```typescript
import { Result, ok, err } from "@bgql/client";

// Repository method
async findById(id: UserId): Promise<Result<User, UserNotFoundError>> {
  const user = this.users.get(id);
  return user ? ok(user) : err(new UserNotFoundError(id));
}

// Resolver converts Result to union
const result = await ctx.services.userQuery.getUser({ id });
if (result.ok) {
  return { __typename: "User", ...result.value };
}
return { __typename: "NotFoundError", ...result.error.toGraphQL() };
```

## DataLoader Pattern

Automatic batching prevents N+1 queries:

```typescript
// Multiple user.load() calls are batched
const users = await Promise.all(
  userIds.map(id => ctx.loaders.user.load(id))
);
// Results in single: SELECT * FROM users WHERE id IN (...)
```

## Sample Data

**Users:**
- `user_1`: Alice Johnson (alice@example.com) - Admin
- `user_2`: Bob Smith (bob@example.com) - Moderator
- `user_3`: Carol Williams (carol@example.com) - User

**Posts:**
- `post_1`: "Introduction to BGQL" (Published, by Alice)
- `post_2`: "Schema-First Development" (Published, by Alice)
- `post_3`: "DataLoaders Explained" (Published, by Bob)
- `post_4`: "Draft: Typed Errors" (Draft, by Alice)
- `post_5`: "TypeScript and GraphQL" (Published, by Carol)

## License

MIT
