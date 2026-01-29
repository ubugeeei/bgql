# Opaque Types

Opaque types (also called branded types or newtypes) provide type-safe wrappers around primitive types, preventing accidental misuse.

## Why Opaque Types?

### The Problem

Without opaque types, all IDs are interchangeable:

```typescript
// Plain strings - easy to mix up
function getUser(userId: string): User { ... }
function getPost(postId: string): Post { ... }

const userId = "user-123";
const postId = "post-456";

// Bug: Wrong ID passed, but TypeScript allows it
getUser(postId);  // No error, but wrong!
```

### The Solution

```graphql
# Better GraphQL schema
opaque UserId = ID
opaque PostId = ID

type User {
  id: UserId
  posts: List<Post>
}

type Post {
  id: PostId
  authorId: UserId
}
```

```typescript
// Generated TypeScript - IDs are distinct types
type UserId = string & { readonly __brand: 'UserId' };
type PostId = string & { readonly __brand: 'PostId' };

function getUser(userId: UserId): User { ... }
function getPost(postId: PostId): Post { ... }

const userId = "user-123" as UserId;
const postId = "post-456" as PostId;

getUser(postId);  // TypeScript error!
getUser(userId);  // OK
```

## Defining Opaque Types

### Basic Syntax

```graphql
# Syntax: opaque TypeName = BaseType
opaque UserId = ID
opaque Email = String
opaque Timestamp = Int
opaque Money = Float
```

### With Validation

```graphql
opaque Email = String @email
opaque PositiveInt = Int @min(1)
opaque Percentage = Float @min(0) @max(100)
opaque Slug = String @pattern("^[a-z0-9-]+$")
```

### Documentation

```graphql
"""
Unique identifier for a user.
Format: "usr_" followed by 24 alphanumeric characters.
"""
opaque UserId = ID

"""
Email address that has been validated and normalized.
Always lowercase, trimmed of whitespace.
"""
opaque Email = String @email
```

## Common Patterns

### Entity IDs

```graphql
# Define ID types for each entity
opaque UserId = ID
opaque PostId = ID
opaque CommentId = ID
opaque TeamId = ID
opaque ProjectId = ID

type User {
  id: UserId
  teamId: TeamId
}

type Post {
  id: PostId
  authorId: UserId
  projectId: ProjectId
}

type Comment {
  id: CommentId
  postId: PostId
  authorId: UserId
}
```

### Validated Strings

```graphql
opaque Email = String @email
opaque URL = String @url
opaque PhoneNumber = String @pattern("^\\+[1-9]\\d{1,14}$")
opaque Slug = String @pattern("^[a-z0-9]+(?:-[a-z0-9]+)*$")
opaque Username = String @minLength(3) @maxLength(20) @pattern("^[a-zA-Z0-9_]+$")
```

### Numeric Types

```graphql
opaque PositiveInt = Int @min(1)
opaque NonNegativeInt = Int @min(0)
opaque Percentage = Float @min(0) @max(100)
opaque Money = Int  # Store as cents for precision
opaque Latitude = Float @min(-90) @max(90)
opaque Longitude = Float @min(-180) @max(180)
```

### Timestamps

```graphql
opaque UnixTimestamp = Int
opaque ISODateTime = String @pattern("^\\d{4}-\\d{2}-\\d{2}T\\d{2}:\\d{2}:\\d{2}")
```

## TypeScript Integration

### Generated Types

```typescript
// Generated from schema
export type UserId = string & { readonly __brand: 'UserId' };
export type PostId = string & { readonly __brand: 'PostId' };
export type Email = string & { readonly __brand: 'Email' };
export type Money = number & { readonly __brand: 'Money' };
```

### Creating Branded Values

```typescript
import { brand } from '@bgql/client';

// Create branded values
const userId = brand<UserId>('usr_abc123');
const email = brand<Email>('user@example.com');

// Or use type assertion (less safe)
const userId = 'usr_abc123' as UserId;
```

### Type-Safe Functions

```typescript
// Functions with opaque types
function getUser(id: UserId): Promise<User> { ... }
function sendEmail(to: Email, subject: string): Promise<void> { ... }

// Usage
const user = await getUser(userId);  // OK
const user = await getUser(postId);  // Type error!

await sendEmail(email, 'Hello');     // OK
await sendEmail('raw@email.com', 'Hello');  // Type error - not branded
```

### Extracting Base Type

```typescript
import type { Unbranded } from '@bgql/client';

type RawUserId = Unbranded<UserId>;  // string
type RawMoney = Unbranded<Money>;    // number

// Useful for JSON serialization
function serialize<T>(value: T): Unbranded<T> {
  return value as Unbranded<T>;
}
```

## Resolver Implementation

### Returning Opaque Types

```typescript
const resolvers = {
  Query: {
    user: async (_, { id }: { id: UserId }, ctx) => {
      // id is typed as UserId
      const user = await ctx.db.users.findById(id);
      return user;
    },
  },

  User: {
    // Field resolvers receive properly typed parent
    posts: async (user: User, _, ctx) => {
      // user.id is UserId
      return ctx.db.posts.findByAuthorId(user.id);
    },
  },
};
```

### Input Validation

```typescript
import { brand, isValidEmail, isValidUserId } from '@bgql/server';

const resolvers = {
  Mutation: {
    createUser: async (_, { input }, ctx) => {
      // Validate and brand the email
      if (!isValidEmail(input.email)) {
        return { __typename: 'ValidationError', field: 'email', message: 'Invalid email' };
      }

      const user = await ctx.db.users.create({
        ...input,
        email: brand<Email>(input.email),
      });

      return { __typename: 'User', ...user };
    },
  },
};
```

## Database Integration

### With Prisma

```typescript
// Prisma schema stores as regular strings
// model User {
//   id    String @id
//   email String @unique
// }

// Repository layer handles branding
class UserRepository {
  async findById(id: UserId): Promise<User | null> {
    const user = await prisma.user.findUnique({
      where: { id: id as string },  // Unbrand for query
    });

    if (!user) return null;

    return {
      ...user,
      id: user.id as UserId,        // Brand the result
      email: user.email as Email,
    };
  }
}
```

### With TypeORM

```typescript
import { Column, Entity, PrimaryColumn } from 'typeorm';

@Entity()
class UserEntity {
  @PrimaryColumn()
  id: string;  // Stored as string

  @Column()
  email: string;
}

// Transform when loading
function toUser(entity: UserEntity): User {
  return {
    id: entity.id as UserId,
    email: entity.email as Email,
  };
}
```

## Relationships

### Cross-Entity References

```graphql
opaque UserId = ID
opaque TeamId = ID

type User {
  id: UserId
  teamId: TeamId
  team: Team
}

type Team {
  id: TeamId
  members: List<User>
}

type Query {
  user(id: UserId): User
  team(id: TeamId): Team
  # Type error if you try: user(id: TeamId)
}
```

### Type-Safe Mutations

```graphql
input AddUserToTeamInput {
  userId: UserId
  teamId: TeamId
}

type Mutation {
  addUserToTeam(input: AddUserToTeamInput): User | NotFoundError
}
```

```typescript
// Client code - can't mix up IDs
await client.mutate(AddUserToTeamDocument, {
  input: {
    userId: user.id,   // Must be UserId
    teamId: team.id,   // Must be TeamId
  },
});
```

## Best Practices

### 1. Use for All Entity IDs

```graphql
# Good: Each entity has its own ID type
opaque UserId = ID
opaque PostId = ID
opaque CommentId = ID

# Avoid: Using plain ID
type User {
  id: ID  # Not type-safe
}
```

### 2. Name Clearly

```graphql
# Good: Clear what the type represents
opaque UserId = ID
opaque EmailAddress = String @email
opaque MoneyInCents = Int

# Avoid: Ambiguous names
opaque Id = ID
opaque Str = String
```

### 3. Add Validation Where Appropriate

```graphql
# Good: Validation at the type level
opaque Email = String @email
opaque PositiveInt = Int @min(1)

# This ensures all Email values are valid emails
```

### 4. Document Formats

```graphql
"""
User ID in the format "usr_" followed by 24 alphanumeric characters.
Example: usr_507f1f77bcf86cd799439011
"""
opaque UserId = ID
```

### 5. Consider Serialization

```typescript
// When sending to external APIs, you may need to unbrand
const rawId: string = userId as string;

// Or use a helper
import { unbrand } from '@bgql/client';
const rawId = unbrand(userId);
```

## Migration

### From Plain Types

```graphql
# Before
type User {
  id: ID
  email: String
}

# After
opaque UserId = ID
opaque Email = String @email

type User {
  id: UserId
  email: Email
}
```

### Gradual Adoption

```typescript
// Helper to convert existing code
function asUserId(id: string): UserId {
  return id as UserId;
}

// Use in migration period
const userId = asUserId(legacyId);
```

## Next Steps

- [Type System](/guide/type-system)
- [Types](/schema/types)
- [Inputs](/schema/inputs)
