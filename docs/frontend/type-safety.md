# Type Safety

Better GraphQL provides end-to-end type safety from schema to client code.

## TypedDocumentNode

The foundation of type safety is `TypedDocumentNode`:

```typescript
interface TypedDocumentNode<TData, TVariables> {
  readonly __resultType?: TData;
  readonly __variablesType?: TVariables;
}
```

### Manual Typing

```typescript
import { gql } from '@bgql/client';

const GetUserDocument = gql<
  { user: { id: string; name: string } | null },
  { id: string }
>`
  query GetUser($id: ID!) {
    user(id: $id) {
      id
      name
    }
  }
`;
```

### Generated Types (Recommended)

```bash
bgql codegen ./schema.bgql --lang typescript -o ./generated/graphql.ts
```

```typescript
// generated/graphql.ts
export const GetUserDocument: TypedDocumentNode<
  GetUserQuery,
  GetUserQueryVariables
>;

export interface GetUserQuery {
  readonly user: User | NotFoundError;
}

export interface GetUserQueryVariables {
  readonly id: string;
}
```

## Type Inference

### Full Inference Chain

```typescript
import { GetUserDocument } from './generated/graphql';

// Variables are typed
const result = await client.queryTyped(
  GetUserDocument,
  { id: '1' }  // Must be { id: string }
);

// Result is typed
if (result.ok) {
  // result.value.user is User | NotFoundError
  if (result.value.user.__typename === 'User') {
    console.log(result.value.user.name);  // string
  }
}
```

### ResultOf and VariablesOf

Extract types from documents:

```typescript
import type { ResultOf, VariablesOf } from '@bgql/client';
import { GetUserDocument } from './generated/graphql';

type GetUserData = ResultOf<typeof GetUserDocument>;
// { user: User | NotFoundError }

type GetUserVars = VariablesOf<typeof GetUserDocument>;
// { id: string }
```

## Discriminated Unions

### __typename Discrimination

All union types include `__typename` for type narrowing:

```typescript
type UserResult =
  | { __typename: 'User'; id: string; name: string }
  | { __typename: 'NotFoundError'; message: string }
  | { __typename: 'AuthError'; message: string };

function handleResult(result: UserResult) {
  switch (result.__typename) {
    case 'User':
      // TypeScript knows result is User
      console.log(result.name);
      break;
    case 'NotFoundError':
      // TypeScript knows result is NotFoundError
      console.log(result.message);
      break;
    case 'AuthError':
      console.log(result.message);
      break;
  }
}
```

### Type Guards

```typescript
import { isTypename } from '@bgql/client';

if (isTypename('User')(result.user)) {
  // result.user is narrowed to User
  console.log(result.user.name);
}
```

### Exhaustive Matching

```typescript
import { matchUnion, assertNever } from '@bgql/client';

// Pattern matching ensures all cases handled
matchUnion(result.user, {
  User: (user) => console.log(user.name),
  NotFoundError: (err) => console.log(err.message),
  AuthError: (err) => console.log(err.message),
});

// Or with switch
function handle(result: UserResult): string {
  switch (result.__typename) {
    case 'User': return result.name;
    case 'NotFoundError': return result.message;
    case 'AuthError': return result.message;
    default: return assertNever(result);  // Compile error if case missing
  }
}
```

## Branded Types

### ID Safety

Prevent mixing different ID types:

```typescript
// Schema
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
// Generated TypeScript
type UserId = string & { readonly __brand: 'UserId' };
type PostId = string & { readonly __brand: 'PostId' };

// Type-safe functions
function getUser(id: UserId): Promise<User> { ... }
function getPost(id: PostId): Promise<Post> { ... }

// Usage
const userId = '123' as UserId;
const postId = '456' as PostId;

getUser(userId);  // ✅ OK
getUser(postId);  // ❌ Type error
```

### Creating Branded Values

```typescript
import { brand } from '@bgql/client';

const userId = brand<string, 'UserId'>(response.id);
const postId = brand<string, 'PostId'>(response.id);
```

## Option Type

### Type-Safe Nullability

```typescript
// Schema
type User {
  id: ID
  name: String
  bio: Option<String>
}
```

```typescript
// Generated TypeScript
interface User {
  readonly id: string;
  readonly name: string;        // Never null
  readonly bio: string | null;  // Can be null
}

// Type-safe access
const name = user.name;         // string (guaranteed)
const bio = user.bio ?? '';     // Need to handle null
```

### Option Helpers

```typescript
import { isSome, isNone, mapOption, unwrapOption } from '@bgql/client';

// Type guards
if (isSome(user.bio)) {
  console.log(user.bio);  // string (not null)
}

// Transform if present
const upperBio = mapOption(user.bio, bio => bio.toUpperCase());

// Unwrap with default
const bio = unwrapOption(user.bio, 'No bio provided');
```

## Result Type

### Type-Safe Error Handling

```typescript
type Result<T, E> = Ok<T> | Err<E>;

interface Ok<T> {
  readonly ok: true;
  readonly value: T;
}

interface Err<E> {
  readonly ok: false;
  readonly error: E;
}
```

### Using Results

```typescript
import { isOk, isErr, match, unwrap, unwrapOr } from '@bgql/client';

const result = await client.query(GetUser, { id: '1' });

// Type guards
if (isOk(result)) {
  console.log(result.value);  // Data type
}

if (isErr(result)) {
  console.log(result.error);  // Error type
}

// Pattern matching
const message = match(result, {
  ok: (data) => `Hello, ${data.user.name}!`,
  err: (error) => `Error: ${error.message}`,
});

// Unwrapping (throws on error)
const data = unwrap(result);

// Unwrap with default
const data = unwrapOr(result, { user: null });
```

## Strict Input Types

### No Extra Properties

```typescript
// Schema
input CreateUserInput {
  name: String
  email: String
}
```

```typescript
// Generated TypeScript
interface CreateUserInput {
  readonly name: string;
  readonly email: string;
}

// Usage
const input: CreateUserInput = {
  name: 'John',
  email: 'john@example.com',
  typo: 'value',  // ❌ Type error: 'typo' does not exist
};
```

### Required vs Optional

```typescript
// Schema
input UpdateUserInput {
  name: Option<String>
  email: Option<String>
}
```

```typescript
// Generated TypeScript
interface UpdateUserInput {
  readonly name?: string | null;
  readonly email?: string | null;
}

// Can omit optional fields
const input: UpdateUserInput = { name: 'New Name' };  // ✅ OK
```

## Fragment Types

### Type-Safe Fragments

```typescript
// Schema fragment
fragment UserFields on User {
  id
  name
  email
}
```

```typescript
// Generated TypeScript
interface UserFieldsFragment {
  readonly __typename: 'User';
  readonly id: string;
  readonly name: string;
  readonly email: string;
}

// Use in components
function UserCard({ user }: { user: UserFieldsFragment }) {
  return <div>{user.name}</div>;
}
```

### Fragment Composition

```typescript
// Compose fragments
type FullUser = UserFieldsFragment & UserProfileFragment;

function UserProfile({ user }: { user: FullUser }) {
  return (
    <div>
      <h1>{user.name}</h1>
      <p>{user.bio}</p>
    </div>
  );
}
```

## Scalar Types

### Custom Scalar Mapping

```typescript
// Configure custom scalars
declare module '@bgql/client' {
  interface CustomScalars {
    DateTime: Date;
    JSON: Record<string, unknown>;
    BigInt: bigint;
  }
}
```

### Default Mappings

| Scalar | TypeScript |
|--------|------------|
| `ID` | `string` |
| `String` | `string` |
| `Int` | `number` |
| `Float` | `number` |
| `Boolean` | `boolean` |
| `DateTime` | `string` |
| `JSON` | `unknown` |

## Connection Types

### Type-Safe Pagination

```typescript
interface Connection<T> {
  readonly edges: ReadonlyArray<Edge<T>>;
  readonly pageInfo: PageInfo;
  readonly totalCount?: number;
}

interface Edge<T> {
  readonly node: T;
  readonly cursor: string;
}

interface PageInfo {
  readonly hasNextPage: boolean;
  readonly hasPreviousPage: boolean;
  readonly startCursor?: string;
  readonly endCursor?: string;
}
```

### Using Connections

```typescript
import { extractNodes } from '@bgql/client';

const result = await client.query(GetUsers, { first: 10 });

if (result.ok) {
  // Extract nodes from connection
  const users: User[] = extractNodes(result.value.users);

  // Check pagination
  if (result.value.users.pageInfo.hasNextPage) {
    const nextCursor = result.value.users.pageInfo.endCursor;
    // Fetch next page
  }
}
```

## Best Practices

### 1. Always Use Generated Types

```typescript
// ✅ Good
import { GetUserDocument } from './generated/graphql';

// ❌ Avoid
const result = await client.query<any>(...);
```

### 2. Use Strict TypeScript Config

```json
{
  "compilerOptions": {
    "strict": true,
    "noUncheckedIndexedAccess": true,
    "exactOptionalPropertyTypes": true
  }
}
```

### 3. Handle All Union Cases

```typescript
// ✅ Good: Exhaustive handling
matchUnion(result.user, {
  User: ...,
  NotFoundError: ...,
  AuthError: ...,
});
```

### 4. Prefer Type Guards Over Assertions

```typescript
// ✅ Good
if (isTypename('User')(result.user)) {
  console.log(result.user.name);
}

// ❌ Avoid
const user = result.user as User;
console.log(user.name);
```

## Next Steps

- [Queries](/frontend/queries)
- [Mutations](/frontend/mutations)
- [Vue.js Integration](/frontend/vue)
