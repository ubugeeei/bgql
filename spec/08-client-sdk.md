# Better GraphQL Specification - Client SDK Design

## 1. Overview

Better GraphQL client SDKs are designed with four core principles:

1. **Strongly typed responses and errors** - Full type safety at compile time
2. **No throw-based error handling** - Errors are values, not exceptions
3. **Partial Promise runtime** - First-class support for streaming responses
4. **Native AbortController** - Cancellation through standard Web APIs

## 2. Design Principles

### 2.1 Errors as Values

Traditional GraphQL clients often throw exceptions for errors, making error handling unpredictable and requiring try-catch blocks everywhere. Better GraphQL treats errors as first-class values in the type system.

```typescript
// Traditional approach (avoid this)
try {
  const user = await client.getUser({ id: "1" });
  // user might be null, might have partial data, might throw
} catch (e) {
  // What type is e? Network error? GraphQL error? Unknown
}

// Better GraphQL approach
const result = await client.getUser({ id: "1" });

// Type-safe switch-based pattern matching
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

### 2.2 Result Type Pattern

All operations return a discriminated union type that includes all possible outcomes.

```typescript
// Generated types from schema
type UserResult =
  | { __typename: "User"; id: UserId; name: string; email: string }
  | { __typename: "NotFoundError"; message: string; resourceId: string }
  | { __typename: "UnauthorizedError"; message: string };

// Client method signature
interface Client {
  getUser(args: { id: UserId }): Promise<UserResult>;
}
```

### 2.3 No Implicit Nullability

Unlike traditional GraphQL where null can appear anywhere in the response tree when errors occur, Better GraphQL's non-nullable-by-default approach means:

- If a field is typed as `String`, it will always be a string (never null)
- If a field might be absent, it's explicitly typed as `Option<String>`
- Errors are explicit union members, not implicit nulls

## 3. Advanced Type Inference

### 3.1 Query-Driven Type Inference

The SDK infers types directly from your queries, ensuring perfect alignment between what you request and what you receive:

```typescript
// The query defines the exact shape of the response
const GetUserQuery = bgql`
  query GetUser($id: UserId!) {
    user(id: $id) {
      ... on User {
        id
        name
        email
        posts(first: 5) {
          edges {
            node {
              id
              title
            }
          }
        }
      }
      ... on NotFoundError {
        message
        resourceId
      }
    }
  }
` as const;

// Type is inferred from the query structure
type GetUserResult = InferQueryResult<typeof GetUserQuery>;
// Equivalent to:
// type GetUserResult =
//   | {
//       readonly __typename: "User";
//       readonly id: UserId;
//       readonly name: string;
//       readonly email: string;
//       readonly posts: {
//         readonly edges: ReadonlyArray<{
//           readonly node: { readonly id: PostId; readonly title: string };
//         }>;
//       };
//     }
//   | { readonly __typename: "NotFoundError"; readonly message: string; readonly resourceId: string };

// Variables are also inferred
type GetUserVariables = InferQueryVariables<typeof GetUserQuery>;
// Equivalent to: { id: UserId }
```

### 3.2 Selection Set Type Inference

Types are inferred based on exactly what you select:

```typescript
// Minimal selection
const MinimalUserQuery = bgql`
  query { user(id: $id) { ... on User { id } } }
` as const;
type MinimalUser = InferQueryResult<typeof MinimalUserQuery>;
// { __typename: "User"; id: UserId }

// Extended selection
const ExtendedUserQuery = bgql`
  query {
    user(id: $id) {
      ... on User {
        id
        name
        email
        bio
        avatarUrl
        createdAt
        posts { totalCount }
      }
    }
  }
` as const;
type ExtendedUser = InferQueryResult<typeof ExtendedUserQuery>;
// {
//   __typename: "User";
//   id: UserId;
//   name: string;
//   email: string;
//   bio: string | null;
//   avatarUrl: string | null;
//   createdAt: DateTime;
//   posts: { totalCount: number };
// }
```

### 3.3 Conditional Type Narrowing

TypeScript's control flow analysis automatically narrows types:

```typescript
const result = await client.getUser({ id });

// Before narrowing: result is UserResult (union of all types)

if (result.__typename === "User") {
  // NARROWED: result is exactly User type
  result.name;    // string (autocomplete works)
  result.email;   // string
  result.id;      // UserId

  // Error: Property 'message' does not exist on type 'User'
  // result.message;
}

if (result.__typename === "NotFoundError") {
  // NARROWED: result is exactly NotFoundError
  result.message;     // string
  result.resourceId;  // string

  // Error: Property 'name' does not exist on type 'NotFoundError'
  // result.name;
}
```

### 3.4 Discriminated Union Inference

The `__typename` field enables perfect discrimination:

```typescript
// Complex union with many types
type SearchResult =
  | { __typename: "User"; id: UserId; name: string; email: string }
  | { __typename: "Post"; id: PostId; title: string; content: string }
  | { __typename: "Comment"; id: CommentId; body: string; authorId: UserId }
  | { __typename: "NotFoundError"; message: string }
  | { __typename: "InvalidQueryError"; message: string; suggestions: string[] };

function handleSearchResult(result: SearchResult) {
  switch (result.__typename) {
    case "User":
      // TypeScript knows: result.name, result.email exist
      return `User: ${result.name} <${result.email}>`;

    case "Post":
      // TypeScript knows: result.title, result.content exist
      return `Post: ${result.title}`;

    case "Comment":
      // TypeScript knows: result.body, result.authorId exist
      return `Comment by ${result.authorId}: ${result.body}`;

    case "NotFoundError":
    case "InvalidQueryError":
      // Both error types have message
      return `Error: ${result.message}`;

    default:
      // Exhaustiveness check - TypeScript error if cases are missing
      const _exhaustive: never = result;
      throw new Error(`Unhandled type: ${_exhaustive}`);
  }
}
```

### 3.5 Generic Type Inference

Client methods use generics that flow through the entire call chain:

```typescript
// Generic client interface
interface Client {
  query<TData, TVariables>(
    document: TypedDocumentNode<TData, TVariables>,
    variables: TVariables,
    options?: QueryOptions
  ): Promise<TData>;

  mutate<TData, TVariables>(
    document: TypedDocumentNode<TData, TVariables>,
    variables: TVariables,
    options?: MutationOptions
  ): Promise<TData>;
}

// Usage - types are fully inferred
const result = await client.query(GetUserDocument, { id: userId });
//    ^? const result: UserResult (inferred from GetUserDocument)

const created = await client.mutate(CreatePostDocument, { input: postInput });
//    ^? const created: CreatePostResult (inferred from CreatePostDocument)
```

### 3.6 Template Literal Type Inference

Operation names are type-safe:

```typescript
// Infer operation name from string literal
type OperationName<T extends string> =
  T extends `query ${infer Name}(${string}` ? Name :
  T extends `query ${infer Name} {${string}` ? Name :
  T extends `mutation ${infer Name}(${string}` ? Name :
  T extends `mutation ${infer Name} {${string}` ? Name :
  never;

// Example
type Name = OperationName<"query GetUser($id: UserId!) { user(id: $id) { id } }">;
// type Name = "GetUser"

// Used for type-safe operation maps
interface OperationRegistry {
  readonly GetUser: { readonly variables: { readonly id: UserId }; readonly result: UserResult };
  readonly CreatePost: { readonly variables: { readonly input: CreatePostInput }; readonly result: CreatePostResult };
}

// Type-safe by operation name
function executeOperation<K extends keyof OperationRegistry>(
  name: K,
  variables: OperationRegistry[K]["variables"]
): Promise<OperationRegistry[K]["result"]>;
```

### 3.7 Utility Type Inference

Rich utility types for working with results:

```typescript
// Extract success type from result
type ExtractSuccess<T> = T extends { __typename: infer N }
  ? N extends `${string}Error` ? never : T
  : never;

type UserSuccess = ExtractSuccess<UserResult>;
// type UserSuccess = { __typename: "User"; id: UserId; name: string; ... }

// Extract all error types from result
type ExtractErrors<T> = T extends { __typename: infer N }
  ? N extends `${string}Error` ? T : never
  : never;

type UserErrors = ExtractErrors<UserResult>;
// type UserErrors = NotFoundError | UnauthorizedError

// Check if result is success
function isSuccess<T extends { __typename: string }>(
  result: T
): result is ExtractSuccess<T> {
  return !result.__typename.endsWith("Error");
}

// Usage
const result = await client.getUser({ id });
if (isSuccess(result)) {
  // result is narrowed to User
  console.log(result.name);
}
```

### 3.8 Mapped Type Inference

Transform types while preserving structure:

```typescript
// Make all fields optional (for patch operations)
type Patch<T> = {
  [K in keyof T]?: T[K] extends object
    ? Patch<T[K]>
    : T[K];
};

type UserPatch = Patch<User>;
// {
//   id?: UserId;
//   name?: string;
//   email?: string;
//   bio?: string | null;
//   ...
// }

// Extract all ID fields
type ExtractIds<T> = {
  [K in keyof T as T[K] extends `${string}Id` ? K : never]: T[K];
};

type UserIds = ExtractIds<User>;
// { id: UserId }

// Deep readonly
type DeepReadonly<T> = {
  readonly [K in keyof T]: T[K] extends object
    ? DeepReadonly<T[K]>
    : T[K];
};
```

### 3.9 Const Assertion and Literal Types

All queries preserve literal types for maximum precision:

```typescript
// Query with const assertion - preserves literal types
const GetUserQuery = {
  operationName: "GetUser",
  variables: { id: "user_123" },
  selections: ["id", "name", "email"] as const,
} as const;

// Type is preserved as literal
type QueryType = typeof GetUserQuery;
// {
//   readonly operationName: "GetUser";
//   readonly variables: { readonly id: "user_123" };
//   readonly selections: readonly ["id", "name", "email"];
// }

// Infer exact fields from selections
type SelectedFields = typeof GetUserQuery.selections[number];
// type SelectedFields = "id" | "name" | "email"
```

### 3.10 Path-Based Type Access

Access nested types with complete type safety:

```typescript
// Deep path type inference
type Path<T, P extends string> =
  P extends `${infer K}.${infer Rest}`
    ? K extends keyof T
      ? Path<T[K], Rest>
      : never
    : P extends keyof T
      ? T[P]
      : never;

// Usage
type User = {
  readonly id: UserId;
  readonly profile: {
    readonly name: string;
    readonly settings: {
      readonly theme: "light" | "dark";
      readonly notifications: {
        readonly email: boolean;
        readonly push: boolean;
      };
    };
  };
};

type Theme = Path<User, "profile.settings.theme">;
// type Theme = "light" | "dark"

type EmailNotification = Path<User, "profile.settings.notifications.email">;
// type EmailNotification = boolean

// Type-safe getter using branded accessor pattern
// The path is validated at compile time via Path<T, P>
type PathAccessor<T, P extends string> = {
  readonly value: Path<T, P>;
  readonly path: P;
};

function createPathAccessor<T>() {
  return function get<P extends ValidPath<T>>(
    obj: T,
    path: P
  ): PathAccessor<T, P> {
    function isRecord(value: unknown): value is Readonly<Record<string, unknown>> {
      return typeof value === "object" && value !== null;
    }

    let current: unknown = obj;
    for (const key of path.split('.')) {
      if (!isRecord(current)) {
        throw new Error(`Invalid path: ${path}`);
      }
      current = current[key];
    }

    // Note: The cast here is safe because:
    // 1. ValidPath<T> constraint ensures only valid paths compile
    // 2. Path<T, P> is computed at compile time from the literal path
    // 3. Runtime traversal follows the same path validated at compile time
    return {
      get value(): Path<T, P> {
        return current as Path<T, P>;
      },
      path,
    } satisfies PathAccessor<T, P>;
  };
}

// ValidPath ensures only valid dot-notation paths compile
type ValidPath<T, Prefix extends string = ""> = T extends object
  ? {
      [K in keyof T & string]: Prefix extends ""
        ? K | ValidPath<T[K], K>
        : `${Prefix}.${K}` | ValidPath<T[K], `${Prefix}.${K}`>;
    }[keyof T & string]
  : never;

// Usage
const getUserPath = createPathAccessor<User>();
const themeAccessor = getUserPath(user, "profile.settings.theme");
//    ^? PathAccessor<User, "profile.settings.theme">
const theme = themeAccessor.value;
//    ^? "light" | "dark"

const user: User = /* ... */;
const theme = get(user, "profile.settings.theme");
//    ^? const theme: "light" | "dark"
```

### 3.11 Recursive Fragment Type Inference

Fragments compose with full type inference:

```typescript
// Fragment type inference
type FragmentResult<F extends FragmentDefinition> =
  F extends FragmentDefinition<infer T, infer S>
    ? Pick<T, S[number] & keyof T> & {
        [K in S[number] as K extends keyof T
          ? T[K] extends object ? K : never
          : never
        ]: T[K] extends Array<infer U>
          ? Array<FragmentResult<ExtractNestedFragment<F, K>>>
          : FragmentResult<ExtractNestedFragment<F, K>>;
      }
    : never;

// Example with nested fragments
const UserBasicFragment = defineFragment({
  on: "User",
  fields: ["id", "name"] as const,
});

const UserWithPostsFragment = defineFragment({
  on: "User",
  fields: ["id", "name", "email"] as const,
  nested: {
    posts: {
      fields: ["id", "title", "content"] as const,
      nested: {
        author: UserBasicFragment,
      },
    },
  },
});

type UserWithPosts = FragmentResult<typeof UserWithPostsFragment>;
// {
//   readonly id: UserId;
//   readonly name: string;
//   readonly email: string;
//   readonly posts: ReadonlyArray<{
//     readonly id: PostId;
//     readonly title: string;
//     readonly content: string;
//     readonly author: { readonly id: UserId; readonly name: string };
//   }>;
// }
```

### 3.12 Variadic Tuple Types for Batch Operations

Batch operations with preserved per-item types:

```typescript
// Variadic tuple for batch queries
type BatchResult<T extends readonly unknown[]> = {
  [K in keyof T]: T[K] extends { variables: infer V; result: infer R }
    ? Awaited<R>
    : never;
};

// Type-safe batch function
function batch<T extends readonly QueryConfig[]>(
  ...queries: T
): Promise<BatchResult<T>>;

// Usage - each result maintains its specific type
const [user, posts, comments] = await batch(
  { query: GetUserDocument, variables: { id: userId } },
  { query: GetPostsDocument, variables: { first: 10 } },
  { query: GetCommentsDocument, variables: { postId } },
);

// Types are preserved individually:
// user: UserResult
// posts: PostConnection
// comments: CommentConnection
```

### 3.13 Conditional Inference Chain

Complex conditional type inference:

```typescript
// Infer the most specific type through a chain
type InferFieldType<
  Schema,
  TypeName extends string,
  FieldName extends string
> = TypeName extends keyof Schema
  ? Schema[TypeName] extends { fields: infer Fields }
    ? FieldName extends keyof Fields
      ? Fields[FieldName] extends { type: infer T }
        ? T extends { kind: "SCALAR"; name: infer S }
          ? ScalarTypeMap[S & keyof ScalarTypeMap]
          : T extends { kind: "OBJECT"; name: infer O }
            ? InferObjectType<Schema, O & string>
            : T extends { kind: "LIST"; ofType: infer I }
              ? Array<InferFieldType<Schema, TypeName, FieldName>>
              : T extends { kind: "NON_NULL"; ofType: infer N }
                ? NonNullable<InferFieldType<Schema, TypeName, FieldName>>
                : unknown
        : never
      : never
    : never
  : never;

// Scalar type mapping - readonly
interface ScalarTypeMap {
  readonly String: string;
  readonly Int: number;
  readonly Float: number;
  readonly Boolean: boolean;
  readonly ID: string;
  readonly DateTime: Date;
  readonly UserId: UserId;
  readonly PostId: PostId;
}
```

### 3.14 Higher-Kinded Type Patterns

Simulate higher-kinded types for maximum flexibility:

```typescript
// Type-level function application
interface TypeRegistry {
  Option: { type: <T>() => T | null };
  List: { type: <T>() => T[] };
  Deferred: { type: <T>() => Deferred<T> };
  NonNull: { type: <T>() => NonNullable<T> };
}

type Apply<F extends keyof TypeRegistry, T> =
  ReturnType<TypeRegistry[F]["type"]<T>>;

// Usage
type MaybeUser = Apply<"Option", User>;
// type MaybeUser = User | null

type UserList = Apply<"List", User>;
// type UserList = User[]

type DeferredUser = Apply<"Deferred", User>;
// type DeferredUser = Deferred<User>

// Compose type functions
type DeferredOptionalList<T> = Apply<"Deferred", Apply<"Option", Apply<"List", T>>>;
// type DeferredOptionalList<User> = Deferred<User[] | null>
```

### 3.15 Type-Level Validation

Compile-time validation of types:

```typescript
// Type-level assertions
type Assert<T extends true> = T;
type IsEqual<A, B> = (<T>() => T extends A ? 1 : 2) extends
  (<T>() => T extends B ? 1 : 2) ? true : false;

// Validate that generated types match expected shape
type _ValidateUserHasId = Assert<IsEqual<User["id"], UserId>>;
type _ValidateUserHasName = Assert<IsEqual<User["name"], string>>;

// Validate newtype distinctness
type _UserIdNotPostId = Assert<
  IsEqual<UserId, PostId> extends true ? false : true
>;

// Validate union exhaustiveness
type AllResultTypes = UserResult["__typename"];
type _HasUser = Assert<"User" extends AllResultTypes ? true : false>;
type _HasNotFound = Assert<"NotFoundError" extends AllResultTypes ? true : false>;
```

### 3.16 Branded Primitives with Compile-Time Validation

Ultra-strict branded types:

```typescript
// Branded types are generated from schema newtypes
// The SDK generates these with proper branding - no manual assertion needed

// generated/types.ts - Auto-generated from schema
declare const BrandSymbol: unique symbol;

interface Branded<T, Brand extends string> {
  readonly value: T;
  readonly [BrandSymbol]: Brand;
}

// Generated newtype definitions
interface UserId extends Branded<string, "UserId"> {}
interface PostId extends Branded<string, "PostId"> {}
interface EmailAddress extends Branded<string, "EmailAddress"> {}
interface Url extends Branded<string, "Url"> {}
interface PositiveInt extends Branded<number, "PositiveInt"> {}

// Generated constructors with validation
// SDK generates these - return type is inferred, no assertion needed
const UserId = Object.assign(
  (value: string): UserId => {
    if (!value.startsWith("user_")) {
      throw new Error("UserId must start with 'user_'");
    }
    return { value, [BrandSymbol]: "UserId" } satisfies UserId;
  },
  { brand: "UserId" } as const
);

const EmailAddress = Object.assign(
  (value: string): EmailAddress => {
    if (!/^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(value)) {
      throw new Error("Invalid email format");
    }
    return { value, [BrandSymbol]: "EmailAddress" } satisfies EmailAddress;
  },
  { brand: "EmailAddress" } as const
);

const PositiveInt = Object.assign(
  (value: number): PositiveInt => {
    if (!Number.isInteger(value) || value <= 0) {
      throw new Error("Must be a positive integer");
    }
    return { value, [BrandSymbol]: "PositiveInt" } satisfies PositiveInt;
  },
  { brand: "PositiveInt" } as const
);

// Type-safe operations - types fully inferred
declare function getUser(id: UserId): Promise<User>;
declare function sendEmail(to: EmailAddress): Promise<void>;

const userId = UserId("user_123");      // UserId (inferred)
const postId = PostId("post_456");      // PostId (inferred)
const email = EmailAddress("user@example.com"); // EmailAddress (inferred)

getUser(userId);     // OK
getUser(postId);     // Error: Argument of type 'PostId' is not assignable to 'UserId'
getUser("raw");      // Error: Argument of type 'string' is not assignable to 'UserId'
sendEmail(email);    // OK
sendEmail("raw");    // Error: Argument of type 'string' is not assignable to 'EmailAddress'
```

### 3.17 Exact Object Types

Prevent excess property errors:

```typescript
// Exact type helper
type Exact<T, U extends T> = T & {
  [K in Exclude<keyof U, keyof T>]: never;
};

// Type-safe input validation
function createUser<T extends CreateUserInput>(
  input: Exact<CreateUserInput, T>
): Promise<User>;

// Usage
createUser({
  name: "John",
  email: "john@example.com",
  password: "secret123",
}); // OK

createUser({
  name: "John",
  email: "john@example.com",
  password: "secret123",
  extraField: "not allowed",  // Error: 'extraField' does not exist in type
});
```

### 3.18 Inference from GraphQL AST

Parse GraphQL at type level:

```typescript
// Type-level GraphQL parser
type ParseQuery<Q extends string> =
  Q extends `query ${infer Name}(${infer Vars}) { ${infer Body} }`
    ? {
        kind: "query";
        name: Name;
        variables: ParseVariables<Vars>;
        selections: ParseSelections<Body>;
      }
    : Q extends `mutation ${infer Name}(${infer Vars}) { ${infer Body} }`
      ? {
          kind: "mutation";
          name: Name;
          variables: ParseVariables<Vars>;
          selections: ParseSelections<Body>;
        }
      : never;

type ParseVariables<V extends string> =
  V extends `$${infer Name}: ${infer Type}, ${infer Rest}`
    ? { name: Name; type: Type } | ParseVariables<Rest>
    : V extends `$${infer Name}: ${infer Type}`
      ? { name: Name; type: Type }
      : never;

// Usage - type is inferred from query string
type Query = ParseQuery<"query GetUser($id: UserId!) { user(id: $id) { id name } }">;
// {
//   kind: "query";
//   name: "GetUser";
//   variables: { name: "id"; type: "UserId!" };
//   selections: ...
// }
```

## 4. Type Generation

### 4.1 Operation Types

The SDK generates types for each operation:

```typescript
// From query:
// query GetUser($id: UserId) {
//   user(id: $id) {
//     ... on User { id name email }
//     ... on NotFoundError { message resourceId }
//   }
// }

// Generated types:
interface GetUserVariables {
  id: UserId;
}

type GetUserResult =
  | { __typename: "User"; id: UserId; name: string; email: string }
  | { __typename: "NotFoundError"; message: string; resourceId: string };

// Generated client method:
getUser(variables: GetUserVariables): Promise<GetUserResult>;
```

### 3.2 Newtype Support

Newtypes are preserved as branded types in TypeScript with validated constructors:

```typescript
// From schema:
// newtype UserId = ID
// newtype PostId = ID

// Generated branded types - see Section 3.16 for full implementation
// SDK generates both the type and constructor

// Compile-time safety with constructor pattern (no type assertions):
declare const UserId: (value: string) => UserId;
declare const PostId: (value: string) => PostId;

declare function getUser(id: UserId): Promise<UserResult>;
declare function getPost(id: PostId): Promise<PostResult>;

const userId = UserId("user_1");  // UserId (inferred, validated at runtime)
const postId = PostId("post_1");  // PostId (inferred, validated at runtime)

getUser(userId);  // OK
getUser(postId);  // Compile error: PostId is not assignable to UserId
getUser("raw");   // Compile error: string is not assignable to UserId
```

### 3.3 Error Type Narrowing

Generated helper functions for type narrowing:

```typescript
// Generated helpers
function isUser(result: UserResult): result is User {
  return result.__typename === "User";
}

function isNotFoundError(result: UserResult): result is NotFoundError {
  return result.__typename === "NotFoundError";
}

function isError(result: UserResult): result is NotFoundError | UnauthorizedError {
  return result.__typename !== "User";
}

// Usage
const result = await client.getUser({ id });

if (isUser(result)) {
  // result is narrowed to User type
  console.log(result.name);
}

if (isError(result)) {
  // result is narrowed to error union
  console.log(result.message);
}
```

## 5. Partial Promise Runtime

### 5.1 Design Philosophy

Deferred fields are wrapped in Promise-like objects that can be `await`ed. This provides a natural, intuitive API where partial data flows seamlessly with JavaScript's async/await syntax.

```typescript
// Query with @defer:
// query GetUser($id: UserId) {
//   user(id: $id) {
//     ... on User {
//       id
//       name
//       ... @defer(label: "profile") {
//         bio
//         avatarUrl
//       }
//     }
//   }
// }

const user = await client.getUser({ id });

// Immediate fields - direct access
console.log(user.name);  // "John" - available immediately

// Deferred fields - await to resolve
console.log(await user.bio);  // "Software engineer" - waits for defer
```

### 5.2 Deferred<T> Type

Deferred fields are wrapped in a `Deferred<T>` type that is both thenable and provides synchronous access:

```typescript
interface Deferred<T> extends PromiseLike<T> {
  // Thenable - can be awaited
  then<R1, R2>(
    onfulfilled?: (value: T) => R1 | PromiseLike<R1>,
    onrejected?: (reason: any) => R2 | PromiseLike<R2>
  ): Promise<R1 | R2>;

  // Synchronous access to current state
  readonly status: "pending" | "resolved";
  readonly current: T | undefined;

  // Check if resolved
  readonly isResolved: boolean;
}
```

### 5.3 Generated Types

```typescript
// Query with @defer generates:
interface UserResult {
  __typename: "User";
  id: UserId;
  name: string;                      // Immediate
  bio: Deferred<Option<string>>;     // Deferred
  avatarUrl: Deferred<Option<string>>; // Deferred
}

// Without @defer (standard query):
interface UserResultComplete {
  __typename: "User";
  id: UserId;
  name: string;
  bio: Option<string>;
  avatarUrl: Option<string>;
}
```

### 5.4 Basic Usage

```typescript
const user = await client.getUser({ id });

switch (user.__typename) {
  case "User":
    // Immediate fields
    console.log(user.name);

    // Await deferred fields
    const bio = await user.bio;
    console.log(bio);

    // Or use Promise.all for parallel resolution
    const [bioValue, avatarValue] = await Promise.all([
      user.bio,
      user.avatarUrl,
    ]);
    break;

  case "NotFoundError":
    console.log(user.message);
    break;
}
```

### 5.5 Synchronous State Check

```typescript
const user = await client.getUser({ id });

if (user.__typename === "User") {
  // Check if deferred field is already resolved
  if (user.bio.isResolved) {
    console.log("Bio already loaded:", user.bio.current);
  } else {
    console.log("Bio still loading...");
    const bio = await user.bio;
    console.log("Bio loaded:", bio);
  }
}
```

### 5.6 Vue Integration

```typescript
import { ref, shallowRef, computed, type Ref, type ShallowRef } from "vue";

// Composable that handles Deferred<T> fields
function useDeferred<T>(deferred: Deferred<T>): {
  readonly value: ShallowRef<T | undefined>;
  readonly isLoading: Ref<boolean>;
} {
  // Use shallowRef for proper generic type inference without assertion
  const value = shallowRef<T | undefined>(deferred.current);
  const isLoading = ref(!deferred.isResolved);

  if (!deferred.isResolved) {
    deferred.then((resolved) => {
      value.value = resolved;
      isLoading.value = false;
    });
  }

  return { value, isLoading };
}

// Usage in component
// <script setup lang="ts">
import { ref, watchEffect } from "vue";

const props = defineProps<{ userId: UserId }>();

const user = ref<UserResult | null>(null);

watchEffect(async () => {
  user.value = await client.getUser({ id: props.userId });
});

// Computed for deferred fields
const bio = computed(() => {
  if (user.value?.__typename === "User") {
    return useDeferred(user.value.bio);
  }
  return { value: ref(undefined), isLoading: ref(false) };
});

const avatar = computed(() => {
  if (user.value?.__typename === "User") {
    return useDeferred(user.value.avatarUrl);
  }
  return { value: ref(undefined), isLoading: ref(false) };
});
// </script>

// <template>
//   <div v-if="user?.__typename === 'User'">
//     <h1>{{ user.name }}</h1>
//     <Skeleton v-if="bio.isLoading.value" />
//     <p v-else>{{ bio.value.value }}</p>
//     <Skeleton v-if="avatar.isLoading.value" />
//     <Avatar v-else :src="avatar.value.value" />
//   </div>
//   <NotFound v-else />
// </template>
```

### 5.7 Nested Deferred Fields

```typescript
// Query:
// query GetUser($id: UserId) {
//   user(id: $id) {
//     ... on User {
//       name
//       ... @defer {
//         posts(first: 5) {
//           edges {
//             node {
//               title
//               ... @defer {
//                 content
//               }
//             }
//           }
//         }
//       }
//     }
//   }
// }

const user = await client.getUser({ id });

if (user.__typename === "User") {
  console.log(user.name);

  // Await posts (deferred)
  const posts = await user.posts;

  for (const edge of posts.edges) {
    console.log(edge.node.title);

    // Await content (nested deferred)
    const content = await edge.node.content;
    console.log(content);
  }
}
```

### 5.8 Stream List as Async Iterator

For `@stream` directive on lists, use async iteration:

```typescript
interface DeferredList<T> extends Deferred<ReadonlyArray<T>> {
  // Async iteration over items as they arrive
  [Symbol.asyncIterator](): AsyncIterator<T>;

  // Current items (received so far)
  readonly items: ReadonlyArray<T>;

  // Total count (if available)
  readonly totalCount: Option<number>;
}

// Query:
// query GetPosts {
//   posts(first: 50) @stream(initialCount: 10) {
//     edges { node { id title } }
//   }
// }

const result = await client.getPosts();

// Option 1: Await all items
const allEdges = await result.posts.edges;
console.log(allEdges.length);  // 50

// Option 2: Process items as they arrive
for await (const edge of result.posts.edges) {
  console.log(edge.node.title);  // Processes each item as it streams in
}

// Option 3: Access current items synchronously
console.log(result.posts.edges.items.length);  // 10 (initial count)
```

### 5.9 Combining Defer and Stream

```typescript
// Query:
// query GetTimeline {
//   posts @stream(initialCount: 10) {
//     edges {
//       node {
//         title
//         author { name }
//         ... @defer {
//           likesCount
//           commentsCount
//         }
//       }
//     }
//   }
// }

const result = await client.getTimeline();

for await (const edge of result.posts.edges) {
  // Title and author available immediately per item
  console.log(edge.node.title);
  console.log(edge.node.author.name);

  // Stats are deferred per item
  const [likes, comments] = await Promise.all([
    edge.node.likesCount,
    edge.node.commentsCount,
  ]);
  console.log(`${likes} likes, ${comments} comments`);
}
```

### 5.10 Error Handling in Deferred Fields

```typescript
const user = await client.getUser({ id });

if (user.__typename === "User") {
  try {
    const bio = await user.bio;
    console.log(bio);
  } catch (error) {
    // Deferred field failed to load
    console.log("Failed to load bio:", error);
  }

  // Or with status check
  if (user.bio.status === "rejected") {
    console.log("Bio failed to load");
  }
}
```

### 5.11 AbortController Integration

All operations accept an `AbortSignal` for cancellation:

```typescript
// Query with AbortController
const controller = new AbortController();

const user = await client.getUser(
  { id: userId },
  { signal: controller.signal }
);

// Cancel from elsewhere
controller.abort();
```

#### Aborting Deferred Fields

```typescript
const controller = new AbortController();

const user = await client.getUser(
  { id: userId },
  { signal: controller.signal }
);

if (user.__typename === "User") {
  // Immediate fields are already resolved
  console.log(user.name);

  // Aborting cancels pending deferred fields
  setTimeout(() => controller.abort(), 1000);

  try {
    const bio = await user.bio;  // May throw AbortError
    console.log(bio);
  } catch (e) {
    if (e instanceof DOMException && e.name === "AbortError") {
      console.log("Bio fetch was cancelled");
    }
  }
}
```

#### Abort-aware Deferred Type

```typescript
interface Deferred<T> extends PromiseLike<T> {
  // ... other fields ...

  // Check if aborted
  readonly isAborted: boolean;

  // Abort this specific deferred field
  abort(reason?: any): void;
}

// Usage
const user = await client.getUser({ id });

if (user.__typename === "User") {
  // Abort just the bio field, keep others loading
  user.bio.abort();

  // Check abort status
  if (user.bio.isAborted) {
    console.log("Bio was aborted");
  }
}
```

#### Timeout Support

```typescript
// Built-in timeout support
const user = await client.getUser(
  { id: userId },
  { signal: AbortSignal.timeout(5000) }  // 5 second timeout
);

// Combined timeout and manual abort
const controller = new AbortController();
const timeoutId = setTimeout(() => controller.abort(), 5000);

try {
  const user = await client.getUser(
    { id: userId },
    { signal: controller.signal }
  );
  clearTimeout(timeoutId);
} catch (e) {
  if (e instanceof DOMException && e.name === "AbortError") {
    console.log("Request timed out or was cancelled");
  }
}
```

#### AbortSignal.any() for Multiple Signals

```typescript
// Combine multiple abort conditions
const userAbort = new AbortController();
const pageUnload = new AbortController();

window.addEventListener("beforeunload", () => pageUnload.abort());

const user = await client.getUser(
  { id: userId },
  { signal: AbortSignal.any([userAbort.signal, pageUnload.signal]) }
);
```

### 5.12 Complete Type Definition

```typescript
interface Deferred<T> extends PromiseLike<T> {
  // PromiseLike implementation
  then<R1 = T, R2 = never>(
    onfulfilled?: (value: T) => R1 | PromiseLike<R1>,
    onrejected?: (reason: any) => R2 | PromiseLike<R2>
  ): Promise<R1 | R2>;

  // Status
  readonly status: "pending" | "resolved" | "rejected";
  readonly isResolved: boolean;
  readonly isPending: boolean;
  readonly isRejected: boolean;

  // Synchronous access (undefined if pending, throws if rejected)
  readonly current: T | undefined;

  // Subscribe to resolution
  subscribe(callback: (value: T) => void): Unsubscribe;
}

interface DeferredList<T> extends Deferred<ReadonlyArray<T>> {
  // Async iteration
  [Symbol.asyncIterator](): AsyncIterator<T>;

  // Current items
  readonly items: ReadonlyArray<T>;
  readonly totalCount: Option<number>;
  readonly isComplete: boolean;

  // Subscribe to new items
  subscribeItems(callback: (newItems: ReadonlyArray<T>) => void): Unsubscribe;
}

type Unsubscribe = () => void;
```

## 6. Subscriptions with Disposable

### 6.1 Using Declaration

Better GraphQL subscriptions implement the TC39 Disposable protocol, allowing automatic cleanup with TypeScript's `using` declaration:

```typescript
// Subscription automatically disposed when scope ends
{
  using subscription = client.subscribe.postCreated({ authorId });

  for await (const post of subscription) {
    console.log("New post:", post.title);

    if (shouldStop) {
      break;  // Subscription disposed automatically
    }
  }
}
// subscription.dispose() called automatically here
```

### 5.2 Subscription Type Definition

```typescript
interface Subscription<T> extends AsyncIterable<T>, AsyncDisposable {
  // Async iteration over events
  [Symbol.asyncIterator](): AsyncIterator<T>;

  // Explicit disposal (called automatically with `using`)
  [Symbol.asyncDispose](): Promise<void>;

  // Manual unsubscribe (alias for dispose)
  unsubscribe(): Promise<void>;

  // AbortController integration
  readonly signal: AbortSignal;
  abort(reason?: any): void;

  // Connection state
  readonly state: "connecting" | "connected" | "disconnected" | "disposed";
  readonly isConnected: boolean;
  readonly isDisposed: boolean;

  // Reconnection events
  onReconnect(callback: () => void): Unsubscribe;
  onDisconnect(callback: (reason: DisconnectReason) => void): Unsubscribe;
}

type DisconnectReason = {
  readonly code: "NETWORK_ERROR" | "SERVER_CLOSED" | "CLIENT_DISPOSED" | "AUTH_EXPIRED" | "ABORTED";
  readonly message: string;
  readonly retryable: boolean;
};

// Subscription options
interface SubscriptionOptions {
  readonly signal?: AbortSignal;
}
```

### 6.3 Basic Usage

```typescript
// With using declaration (recommended)
async function watchNewPosts(authorId: UserId) {
  using subscription = client.subscribe.postCreated({ authorId });

  for await (const result of subscription) {
    switch (result.__typename) {
      case "Post":
        console.log("New post:", result.title);
        break;
    }
  }
}
// Automatically cleaned up when function returns or throws

// With AbortController
async function watchWithAbort(authorId: UserId, signal: AbortSignal) {
  using subscription = client.subscribe.postCreated(
    { authorId },
    { signal }
  );

  for await (const result of subscription) {
    console.log("New post:", result.title);
  }
}

// Usage
const controller = new AbortController();
watchWithAbort(authorId, controller.signal);

// Cancel from elsewhere
setTimeout(() => controller.abort(), 60000);  // Stop after 1 minute
```

### 6.4 Multiple Subscriptions

```typescript
async function watchActivity(userId: UserId) {
  // All subscriptions disposed together when scope ends
  using postSub = client.subscribe.postCreated({ authorId: userId });
  using commentSub = client.subscribe.commentCreated({ userId });
  using notificationSub = client.subscribe.notification();

  // Process events from multiple subscriptions concurrently
  await Promise.race([
    processSubscription(postSub, handlePost),
    processSubscription(commentSub, handleComment),
    processSubscription(notificationSub, handleNotification),
  ]);
}

async function processSubscription<T>(
  subscription: Subscription<T>,
  handler: (event: T) => void
) {
  for await (const event of subscription) {
    handler(event);
  }
}
```

### 6.5 Vue Integration with Disposable

```typescript
// Composable for subscriptions
function useSubscription<T>(
  subscribeFn: () => Subscription<T>,
  handler: (event: T) => void
) {
  const state = ref<"connecting" | "connected" | "disconnected">("connecting");
  let subscription: Subscription<T> | null = null;

  onMounted(async () => {
    subscription = subscribeFn();

    subscription.onReconnect(() => {
      state.value = "connected";
    });

    subscription.onDisconnect(() => {
      state.value = "disconnected";
    });

    state.value = "connected";

    for await (const event of subscription) {
      handler(event);
    }
  });

  onUnmounted(async () => {
    // Dispose subscription when component unmounts
    await subscription?.[Symbol.asyncDispose]();
  });

  return { state };
}

// Usage in component
// <script setup lang="ts">
const posts = ref<Post[]>([]);

const { state } = useSubscription(
  () => client.subscribe.postCreated({ authorId }),
  (result) => {
    if (result.__typename === "Post") {
      posts.value.unshift(result);
    }
  }
);
// </script>

// <template>
//   <div>
//     <span v-if="state === 'connecting'">Connecting...</span>
//     <span v-else-if="state === 'disconnected'">Reconnecting...</span>
//     <ul>
//       <li v-for="post in posts" :key="post.id">{{ post.title }}</li>
//     </ul>
//   </div>
// </template>
```

### 6.6 Subscription with Typed Events

```typescript
// Subscription schema:
// type Subscription {
//   notification: Notification @requireAuth
// }
//
// type Notification {
//   id: ID
//   type: NotificationType
//   message: String
//   data: Option<JSON>
// }
//
// enum NotificationType { NewFollower, NewComment, NewLike, Mention, System }

async function handleNotifications() {
  using subscription = client.subscribe.notification();

  for await (const notification of subscription) {
    switch (notification.type) {
      case "NewFollower":
        showFollowerNotification(notification);
        break;
      case "NewComment":
        showCommentNotification(notification);
        break;
      case "NewLike":
        showLikeNotification(notification);
        break;
      case "Mention":
        showMentionNotification(notification);
        break;
      case "System":
        showSystemNotification(notification);
        break;
    }
  }
}
```

### 6.7 Reconnection Handling

```typescript
async function robustSubscription(userId: UserId) {
  using subscription = client.subscribe.userUpdated({ userId });

  // Handle reconnection events
  subscription.onReconnect(() => {
    console.log("Reconnected to subscription");
    showToast("Connection restored");
  });

  subscription.onDisconnect((reason) => {
    switch (reason.code) {
      case "NETWORK_ERROR":
        showToast("Connection lost, retrying...");
        break;
      case "AUTH_EXPIRED":
        redirectToLogin();
        break;
      case "SERVER_CLOSED":
        showToast("Server closed connection");
        break;
    }
  });

  for await (const event of subscription) {
    handleUserUpdate(event);
  }
}
```

### 6.8 Conditional Subscription

```typescript
async function conditionalWatch(options: WatchOptions) {
  // Subscription only created if condition is true
  await using subscription = options.watchPosts
    ? client.subscribe.postCreated({})
    : null;

  if (subscription) {
    for await (const post of subscription) {
      console.log(post.title);
    }
  }
}
```

### 5.9 DisposableStack for Complex Cleanup

```typescript
async function complexSubscriptionSetup() {
  // Use DisposableStack for multiple resources
  await using stack = new AsyncDisposableStack();

  const postSub = stack.use(client.subscribe.postCreated({}));
  const commentSub = stack.use(client.subscribe.commentCreated({}));

  // Add custom cleanup
  stack.defer(async () => {
    await saveState();
    console.log("Cleanup complete");
  });

  // All resources disposed in reverse order when scope ends
  await Promise.all([
    handlePostEvents(postSub),
    handleCommentEvents(commentSub),
  ]);
}
```

## 7. Error Handling Patterns

### 7.1 Switch-based Pattern Matching

Use TypeScript's switch statement with `__typename` for exhaustive type-safe handling:

```typescript
const result = await client.getUser({ id });

switch (result.__typename) {
  case "User":
    // result is narrowed to User type
    console.log(`Hello, ${result.name}!`);
    console.log(`Email: ${result.email}`);
    break;

  case "NotFoundError":
    // result is narrowed to NotFoundError type
    console.log(`User ${result.resourceId} not found`);
    break;

  case "UnauthorizedError":
    // result is narrowed to UnauthorizedError type
    console.log(`Please log in: ${result.message}`);
    break;
}
```

### 6.2 Exhaustive Switch with Return

For functions that must handle all cases:

```typescript
function getUserDisplayMessage(result: UserResult): string {
  switch (result.__typename) {
    case "User":
      return `Hello, ${result.name}!`;

    case "NotFoundError":
      return `User ${result.resourceId} not found`;

    case "UnauthorizedError":
      return `Please log in`;

    default:
      // Compile-time exhaustiveness check
      const _exhaustive: never = result;
      throw new Error(`Unhandled case: ${_exhaustive}`);
  }
}
```

### 7.3 Switch with Early Return

Common pattern for handling errors first:

```typescript
async function displayUserProfile(userId: UserId): Promise<void> {
  const result = await client.getUser({ id: userId });

  switch (result.__typename) {
    case "NotFoundError":
      showError(`User not found: ${result.resourceId}`);
      return;

    case "UnauthorizedError":
      redirectToLogin();
      return;

    case "User":
      // Continue with user data
      break;
  }

  // TypeScript knows result is User here
  renderProfile({
    name: result.name,
    email: result.email,
    bio: result.bio,
  });
}
```

### 6.4 Nested Switch for Complex Results

For mutations with multiple success/error cases:

```typescript
const result = await client.createUser({ input });

switch (result.__typename) {
  case "User":
    console.log(`Created user: ${result.id}`);
    redirect(`/users/${result.id}`);
    break;

  case "ValidationError":
    showFieldError(result.field, result.message);
    break;

  case "EmailAlreadyExistsError":
    showError(`Email ${result.existingEmail} is already registered`);
    break;

  case "WeakPasswordError":
    showError(`Password requirements: ${result.requirements.join(", ")}`);
    break;
}
```

### 7.5 Async Result Pipeline

```typescript
// Pipeline for multiple operations
async function pipeline() {
  const userResult = await client.getUser({ id: userId });

  if (!isUser(userResult)) {
    return userResult;  // Return error early
  }

  const postsResult = await client.getUserPosts({
    userId: userResult.id
  });

  if (!isPostConnection(postsResult)) {
    return postsResult;  // Return error early
  }

  return {
    user: userResult,
    posts: postsResult.edges.map(e => e.node),
  };
}
```

## 8. Type-safe Interceptors

### 8.1 Error Interceptor Pattern

Define global error handlers that intercept specific error types before they reach the call site:

```typescript
interface ErrorInterceptor<E> {
  // Error type to intercept
  readonly errorType: E["__typename"];
  // Handler returns true if error was handled (stop propagation)
  handle(error: E): boolean | Promise<boolean>;
}

interface ClientConfig {
  readonly endpoint: string;
  readonly interceptors?: {
    readonly errors?: ReadonlyArray<ErrorInterceptor<any>>;
  };
}
```

### 8.2 Defining Interceptors

```typescript
// Type-safe interceptor for UnauthorizedError
const authInterceptor: ErrorInterceptor<UnauthorizedError> = {
  errorType: "UnauthorizedError",
  handle(error) {
    console.log("Auth failed:", error.message);
    redirectToLogin();
    return true; // Error handled, don't propagate
  },
};

// Type-safe interceptor for RateLimitError
const rateLimitInterceptor: ErrorInterceptor<RateLimitError> = {
  errorType: "RateLimitError",
  handle(error) {
    showToast(`Rate limited. Retry after ${error.retryAfter}s`);
    return true;
  },
};

// Type-safe interceptor for ForbiddenError
const forbiddenInterceptor: ErrorInterceptor<ForbiddenError> = {
  errorType: "ForbiddenError",
  handle(error) {
    showToast(`Permission denied: ${error.requiredPermission}`);
    return false; // Let call site also handle it
  },
};
```

### 8.3 Client Configuration

```typescript
const client = createClient({
  endpoint: "https://api.example.com/graphql",
  interceptors: {
    errors: [
      authInterceptor,
      rateLimitInterceptor,
      forbiddenInterceptor,
    ],
  },
});
```

### 8.4 Interceptor Execution Flow

```
Request → Response → Error Interceptors → Call Site
                           ↓
                    [UnauthorizedError?] → authInterceptor.handle()
                           ↓                      ↓
                    [RateLimitError?]      (handled=true: stop)
                           ↓                      ↓
                    [ForbiddenError?]      (handled=false: continue)
                           ↓
                      Call Site (switch statement)
```

### 8.5 Filtered Result Types

When interceptors handle certain errors, the call site receives a narrowed type:

```typescript
// Without interceptors: full union
type UserResult = User | NotFoundError | UnauthorizedError | ForbiddenError;

// With auth interceptor handling UnauthorizedError:
// The interceptor guarantees UnauthorizedError is handled globally
type UserResultFiltered = User | NotFoundError | ForbiddenError;
```

Generated client with interceptor-aware types:

```typescript
// Client knows which errors are intercepted
interface ClientWithInterceptors {
  // Original method (no interceptors)
  getUser(args: { id: UserId }): Promise<UserResult>;

  // Interceptor-aware method (UnauthorizedError handled globally)
  getUserHandled(args: { id: UserId }): Promise<Exclude<UserResult, UnauthorizedError>>;
}
```

### 8.6 Type-safe Interceptor Builder

```typescript
// Builder pattern for type-safe interceptor configuration
const client = createClient({
  endpoint: "https://api.example.com/graphql",
})
  .intercept("UnauthorizedError", (error) => {
    redirectToLogin();
    return true;
  })
  .intercept("RateLimitError", (error) => {
    // error is typed as RateLimitError
    scheduleRetry(error.retryAfter);
    return true;
  })
  .intercept("ForbiddenError", (error) => {
    // error is typed as ForbiddenError
    showPermissionDenied(error.requiredPermission);
    return false; // Also handle at call site
  });

// Result type automatically excludes intercepted errors
const result = await client.getUser({ id });

// TypeScript knows: result is User | NotFoundError | ForbiddenError
// (UnauthorizedError and RateLimitError are handled by interceptors)
switch (result.__typename) {
  case "User":
    showProfile(result);
    break;
  case "NotFoundError":
    showNotFound(result.resourceId);
    break;
  case "ForbiddenError":
    // Additional handling beyond interceptor
    logSecurityEvent(result);
    break;
}
```

### 8.7 Conditional Interception

```typescript
const client = createClient({
  endpoint: "https://api.example.com/graphql",
})
  .intercept("UnauthorizedError", (error, context) => {
    // Don't intercept on login page
    if (context.operation === "Login") {
      return false; // Let call site handle
    }
    redirectToLogin();
    return true;
  });
```

### 8.8 Async Interceptors

```typescript
const client = createClient({
  endpoint: "https://api.example.com/graphql",
})
  .intercept("UnauthorizedError", async (error) => {
    // Try to refresh token
    const refreshed = await refreshAuthToken();

    if (refreshed) {
      // Token refreshed, retry will happen automatically
      return "retry";
    }

    // Refresh failed, redirect to login
    redirectToLogin();
    return true;
  });
```

### 8.9 Interceptor Return Values

```typescript
type InterceptorResult =
  | true      // Error handled, stop propagation
  | false     // Error not handled, continue to call site
  | "retry";  // Retry the request (e.g., after token refresh)
```

## 9. Network Error Handling

### 9.1 Transport Errors

Network errors are separate from GraphQL errors:

```typescript
type NetworkError = {
  readonly __typename: "NetworkError";
  readonly message: string;
  readonly code: "TIMEOUT" | "CONNECTION_REFUSED" | "DNS_FAILURE" | "UNKNOWN";
  readonly retryable: boolean;
};

type QueryResult<T> = T | NetworkError;

// Client method returns both possible error types
async function getUser(args: { id: UserId }): Promise<QueryResult<UserResult>> {
  // ...
}

// Usage
const result = await client.getUser({ id });

if (result.__typename === "NetworkError") {
  if (result.retryable) {
    // Retry logic
  }
  return;
}

// Now result is UserResult
if (isUser(result)) {
  console.log(result.name);
}
```

### 9.2 Retry Configuration

```typescript
interface RetryConfig {
  readonly maxAttempts: number;
  readonly initialDelay: number;
  readonly maxDelay: number;
  readonly backoffMultiplier: number;
  readonly retryOn: (error: NetworkError) => boolean;
}

const client = createClient({
  endpoint: "https://api.example.com/graphql",
  retry: {
    maxAttempts: 3,
    initialDelay: 1000,
    maxDelay: 10000,
    backoffMultiplier: 2,
    retryOn: (error) => error.retryable,
  },
});
```

## 10. Code Generation

### 10.1 CLI Usage

```bash
# Generate TypeScript client from schema
bgql codegen \
  --schema ./schema.bgql \
  --operations ./operations/**/*.bgql \
  --output ./generated/client.ts \
  --target typescript

# Generate with specific features
bgql codegen \
  --schema ./schema.bgql \
  --operations ./operations/**/*.bgql \
  --output ./generated/client.ts \
  --target typescript \
  --features streaming,vue-composables,match-helpers
```

### 10.2 Configuration File

```yaml
# bgql.config.yaml
schema: ./schema.bgql
operations:
  - ./operations/**/*.bgql
output: ./generated/client.ts
target: typescript

features:
  streaming: true
  vueComposables: true
  matchHelpers: true
  typeGuards: true

typescript:
  strictNullChecks: true
  useUnknownInCatchVariables: true

newtypes:
  UserId: { brand: true }
  PostId: { brand: true }
  EmailAddress: { brand: true, validate: "email" }
```

## 11. Vanilla TypeScript

The SDK provides a framework-agnostic core that works with vanilla TypeScript/JavaScript.

### 11.1 Basic Client Usage

```typescript
import { createClient } from "@better-graphql/client";
import type { UserResult, CreatePostResult, PostCreatedResult } from "./generated/types";

// Create client instance
const client = createClient({
  endpoint: "https://api.example.com/graphql",
});

// Query
async function fetchUser(id: UserId): Promise<void> {
  const result = await client.getUser({ id });

  switch (result.__typename) {
    case "User":
      console.log(`User: ${result.name}`);
      break;
    case "NotFoundError":
      console.error(`Not found: ${result.message}`);
      break;
    case "UnauthorizedError":
      console.error(`Unauthorized: ${result.message}`);
      break;
  }
}

// Mutation
async function createPost(input: CreatePostInput): Promise<void> {
  const result = await client.createPost({ input });

  if (result.__typename === "Post") {
    console.log(`Created: ${result.title}`);
  }
}
```

### 11.2 Observable Pattern

For reactive state management without a framework:

```typescript
// Simple observable implementation
interface Observable<T> {
  subscribe(observer: (value: T) => void): () => void;
  getValue(): T;
}

interface QueryObservable<T> extends Observable<QueryState<T>> {
  readonly refetch: () => Promise<void>;
}

type QueryState<T> = {
  readonly data: T | null;
  readonly loading: boolean;
  readonly error: NetworkError | null;
};

// Generated observable factory
function createUserQuery(
  variables: GetUserVariables
): QueryObservable<UserResult> {
  let state: QueryState<UserResult> = {
    data: null,
    loading: true,
    error: null,
  };
  const subscribers = new Set<(value: QueryState<UserResult>) => void>();

  function notify(): void {
    for (const subscriber of subscribers) {
      subscriber(state);
    }
  }

  async function execute(): Promise<void> {
    state = { ...state, loading: true, error: null };
    notify();

    try {
      const data = await client.getUser(variables);
      state = { data, loading: false, error: null };
    } catch (e: unknown) {
      state = { ...state, loading: false, error: toNetworkError(e) };
    }
    notify();
  }

  // Initial fetch
  execute();

  return {
    subscribe(observer) {
      subscribers.add(observer);
      observer(state); // Immediate callback with current state
      return () => subscribers.delete(observer);
    },
    getValue() {
      return state;
    },
    refetch: execute,
  };
}

// Usage
const userQuery = createUserQuery({ id: UserId("user_123") });

const unsubscribe = userQuery.subscribe((state) => {
  if (state.loading) {
    console.log("Loading...");
  } else if (state.error) {
    console.error("Error:", state.error.message);
  } else if (state.data) {
    console.log("Data:", state.data);
  }
});

// Later: cleanup
unsubscribe();
```

### 11.3 Store Pattern

A simple store for managing multiple queries:

```typescript
type StoreState = {
  readonly users: ReadonlyMap<string, UserResult>;
  readonly posts: ReadonlyMap<string, PostResult>;
};

interface Store {
  readonly state: StoreState;
  subscribe(listener: (state: StoreState) => void): () => void;
  fetchUser(id: UserId): Promise<UserResult>;
  fetchPost(id: PostId): Promise<PostResult>;
  invalidateUser(id: UserId): void;
}

function createStore(): Store {
  let state: StoreState = {
    users: new Map(),
    posts: new Map(),
  };
  const listeners = new Set<(state: StoreState) => void>();

  function notify(): void {
    for (const listener of listeners) {
      listener(state);
    }
  }

  return {
    get state() {
      return state;
    },

    subscribe(listener) {
      listeners.add(listener);
      return () => listeners.delete(listener);
    },

    async fetchUser(id) {
      const result = await client.getUser({ id });
      const users = new Map(state.users);
      users.set(id.value, result);
      state = { ...state, users };
      notify();
      return result;
    },

    async fetchPost(id) {
      const result = await client.getPost({ id });
      const posts = new Map(state.posts);
      posts.set(id.value, result);
      state = { ...state, posts };
      notify();
      return result;
    },

    invalidateUser(id) {
      const users = new Map(state.users);
      users.delete(id.value);
      state = { ...state, users };
      notify();
    },
  };
}

// Usage
const store = createStore();

store.subscribe((state) => {
  console.log("Store updated:", state);
});

await store.fetchUser(UserId("user_123"));
```

### 11.4 Event Emitter Pattern

For event-driven architectures:

```typescript
type EventMap = {
  readonly userFetched: UserResult;
  readonly postCreated: PostResult;
  readonly error: NetworkError;
};

interface TypedEventEmitter<T extends Record<string, unknown>> {
  on<K extends keyof T>(event: K, handler: (data: T[K]) => void): () => void;
  emit<K extends keyof T>(event: K, data: T[K]): void;
}

function createEventEmitter<T extends Record<string, unknown>>(): TypedEventEmitter<T> {
  const handlers = new Map<keyof T, Set<(data: unknown) => void>>();

  return {
    on(event, handler) {
      if (!handlers.has(event)) {
        handlers.set(event, new Set());
      }
      handlers.get(event)!.add(handler as (data: unknown) => void);
      return () => handlers.get(event)?.delete(handler as (data: unknown) => void);
    },

    emit(event, data) {
      handlers.get(event)?.forEach((handler) => handler(data));
    },
  };
}

// Client with events
const events = createEventEmitter<EventMap>();

async function fetchUserWithEvents(id: UserId): Promise<void> {
  try {
    const result = await client.getUser({ id });
    events.emit("userFetched", result);
  } catch (e: unknown) {
    events.emit("error", toNetworkError(e));
  }
}

// Subscribe to events
events.on("userFetched", (user) => {
  console.log("User fetched:", user);
});

events.on("error", (error) => {
  console.error("Error occurred:", error.message);
});
```

### 11.5 Subscription Handling

Subscriptions with vanilla TypeScript:

```typescript
// Subscription manager
interface SubscriptionManager {
  readonly activeCount: number;
  add<T>(
    subscription: AsyncIterable<T> & Disposable,
    handler: (data: T) => void
  ): string;
  remove(id: string): void;
  removeAll(): void;
}

function createSubscriptionManager(): SubscriptionManager {
  const subscriptions = new Map<string, { dispose: () => void }>();
  let idCounter = 0;

  return {
    get activeCount() {
      return subscriptions.size;
    },

    add(subscription, handler) {
      const id = `sub_${++idCounter}`;

      // Start consuming the async iterable
      (async () => {
        try {
          for await (const data of subscription) {
            handler(data);
          }
        } catch (e) {
          console.error("Subscription error:", e);
        } finally {
          subscriptions.delete(id);
        }
      })();

      subscriptions.set(id, {
        dispose: () => subscription[Symbol.dispose](),
      });

      return id;
    },

    remove(id) {
      const sub = subscriptions.get(id);
      if (sub) {
        sub.dispose();
        subscriptions.delete(id);
      }
    },

    removeAll() {
      for (const sub of subscriptions.values()) {
        sub.dispose();
      }
      subscriptions.clear();
    },
  };
}

// Usage
const manager = createSubscriptionManager();

const subId = manager.add(
  client.subscribe.postCreated({ authorId }),
  (result) => {
    if (result.__typename === "Post") {
      console.log("New post:", result.title);
    }
  }
);

// Later: cleanup specific subscription
manager.remove(subId);

// Or cleanup all
manager.removeAll();
```

### 11.6 Deferred Fields Handling

Handle streaming responses without a framework:

```typescript
interface DeferredState<T> {
  readonly isResolved: boolean;
  readonly value: T | undefined;
  readonly error: Error | undefined;
}

interface DeferredHandler<T> {
  readonly state: DeferredState<T>;
  onResolved(callback: (value: T) => void): () => void;
  onError(callback: (error: Error) => void): () => void;
}

function createDeferredHandler<T>(deferred: Deferred<T>): DeferredHandler<T> {
  let state: DeferredState<T> = {
    isResolved: deferred.isResolved,
    value: deferred.current,
    error: undefined,
  };

  const resolvedCallbacks = new Set<(value: T) => void>();
  const errorCallbacks = new Set<(error: Error) => void>();

  if (!deferred.isResolved) {
    deferred
      .then((value) => {
        state = { isResolved: true, value, error: undefined };
        for (const callback of resolvedCallbacks) {
          callback(value);
        }
      })
      .catch((error: Error) => {
        state = { isResolved: true, value: undefined, error };
        for (const callback of errorCallbacks) {
          callback(error);
        }
      });
  }

  return {
    get state() {
      return state;
    },

    onResolved(callback) {
      if (state.isResolved && state.value !== undefined) {
        callback(state.value);
      } else {
        resolvedCallbacks.add(callback);
      }
      return () => resolvedCallbacks.delete(callback);
    },

    onError(callback) {
      if (state.error) {
        callback(state.error);
      } else {
        errorCallbacks.add(callback);
      }
      return () => errorCallbacks.delete(callback);
    },
  };
}

// Usage with streaming query
async function handleUserWithDeferred(id: UserId): Promise<void> {
  const result = await client.getUser({ id });

  if (result.__typename === "User") {
    console.log("Name:", result.name);

    // Handle deferred bio field
    const bioHandler = createDeferredHandler(result.bio);

    if (bioHandler.state.isResolved) {
      console.log("Bio (immediate):", bioHandler.state.value);
    } else {
      console.log("Bio loading...");
      bioHandler.onResolved((bio) => {
        console.log("Bio (deferred):", bio);
      });
    }
  }
}
```

### 11.7 DOM Integration

Direct DOM manipulation with type safety:

```typescript
// Type-safe DOM renderer
interface DOMRenderer<T> {
  render(data: T): void;
  clear(): void;
}

function createUserRenderer(container: HTMLElement): DOMRenderer<UserResult> {
  return {
    render(data) {
      container.innerHTML = ""; // Clear previous content

      switch (data.__typename) {
        case "User": {
          const userDiv = document.createElement("div");
          userDiv.className = "user-card";

          const nameEl = document.createElement("h2");
          nameEl.textContent = data.name;
          userDiv.appendChild(nameEl);

          const emailEl = document.createElement("p");
          emailEl.textContent = data.email;
          userDiv.appendChild(emailEl);

          container.appendChild(userDiv);
          break;
        }

        case "NotFoundError": {
          const errorDiv = document.createElement("div");
          errorDiv.className = "error";
          errorDiv.textContent = `Error: ${data.message}`;
          container.appendChild(errorDiv);
          break;
        }

        case "UnauthorizedError": {
          const errorDiv = document.createElement("div");
          errorDiv.className = "error unauthorized";
          errorDiv.textContent = "Please log in to view this user.";
          container.appendChild(errorDiv);
          break;
        }
      }
    },

    clear() {
      container.innerHTML = "";
    },
  };
}

// Usage
const container = document.getElementById("user-container")!;
const renderer = createUserRenderer(container);

async function displayUser(id: UserId): Promise<void> {
  const result = await client.getUser({ id });
  renderer.render(result);
}
```

### 11.8 AbortController Integration

Cancellable requests with AbortController:

```typescript
interface CancellableQuery<T> {
  readonly promise: Promise<T>;
  readonly cancel: () => void;
  readonly isCancelled: boolean;
}

function createCancellableQuery<T>(
  queryFn: (signal: AbortSignal) => Promise<T>
): CancellableQuery<T> {
  const controller = new AbortController();
  let cancelled = false;

  const promise = queryFn(controller.signal).catch((e: unknown) => {
    if (e instanceof Error && e.name === "AbortError") {
      throw new Error("Query was cancelled");
    }
    throw e;
  });

  return {
    promise,
    cancel() {
      if (!cancelled) {
        cancelled = true;
        controller.abort();
      }
    },
    get isCancelled() {
      return cancelled;
    },
  };
}

// Usage
const query = createCancellableQuery((signal) =>
  client.getUser({ id: UserId("user_123") }, { signal })
);

// Cancel if needed (e.g., user navigates away)
document.getElementById("cancel-btn")?.addEventListener("click", () => {
  query.cancel();
});

try {
  const result = await query.promise;
  console.log("Result:", result);
} catch (e) {
  if (query.isCancelled) {
    console.log("Query was cancelled by user");
  } else {
    console.error("Query failed:", e);
  }
}
```

### 11.9 Batch Operations

Execute multiple queries efficiently:

```typescript
// Type-safe batch executor
interface BatchExecutor {
  add<T>(query: () => Promise<T>): Promise<T>;
  execute(): Promise<void>;
}

function createBatchExecutor(): BatchExecutor {
  const pending: Array<{
    query: () => Promise<unknown>;
    resolve: (value: unknown) => void;
    reject: (error: unknown) => void;
  }> = [];

  return {
    add<T>(query: () => Promise<T>): Promise<T> {
      return new Promise<T>((resolve, reject) => {
        pending.push({
          query,
          resolve: resolve as (value: unknown) => void,
          reject,
        });
      });
    },

    async execute() {
      const batch = [...pending];
      pending.length = 0;

      const results = await Promise.allSettled(
        batch.map((item) => item.query())
      );

      results.forEach((result, index) => {
        if (result.status === "fulfilled") {
          batch[index].resolve(result.value);
        } else {
          batch[index].reject(result.reason);
        }
      });
    },
  };
}

// Usage
const batch = createBatchExecutor();

const userPromise = batch.add(() => client.getUser({ id: UserId("user_1") }));
const postPromise = batch.add(() => client.getPost({ id: PostId("post_1") }));

// Execute all at once
await batch.execute();

const [user, post] = await Promise.all([userPromise, postPromise]);
console.log("User:", user);
console.log("Post:", post);
```

## 12. Vue Integration

### 12.1 Generated Composables

The SDK generates type-safe Vue composables with full reactive support:

```typescript
// generated/composables.ts

import { ref, shallowRef, computed, watch, reactive, unref, onUnmounted, type Ref, type ShallowRef, type MaybeRef } from "vue";

/**
 * Type guard for NetworkError
 */
function isNetworkError(e: unknown): e is NetworkError {
  return (
    typeof e === "object" &&
    e !== null &&
    "message" in e &&
    typeof (e as { message: unknown }).message === "string"
  );
}

/**
 * Wrap unknown error into NetworkError safely
 */
function toNetworkError(e: unknown): NetworkError {
  if (isNetworkError(e)) {
    return e;
  }
  return {
    message: e instanceof Error ? e.message : String(e),
    code: "UNKNOWN_ERROR",
  } satisfies NetworkError;
}

/**
 * Query composable with full type inference
 */
export function useGetUser(
  variables: MaybeRef<GetUserVariables>
): UseQueryResult<UserResult> {
  const data = shallowRef<UserResult | null>(null);
  const loading = ref(true);
  const error = shallowRef<NetworkError | null>(null);

  // Full reactive variables support
  watch(
    () => unref(variables),
    async (vars) => {
      loading.value = true;
      error.value = null;
      try {
        data.value = await client.getUser(vars);
      } catch (e: unknown) {
        error.value = toNetworkError(e);
      } finally {
        loading.value = false;
      }
    },
    { immediate: true }
  );

  return { data, loading, error, refetch: () => /* ... */ };
}

/**
 * Streaming query with Deferred support
 */
export function useGetUserStreaming(
  variables: MaybeRef<GetUserVariables>
): UseStreamingQueryResult<UserResult> {
  const data = ref<UserResult | null>(null);
  const loading = ref(true);
  const deferredFields = reactive<DeferredFieldsState>({});

  // Handle deferred fields reactively
  watch(
    () => unref(variables),
    async (vars) => {
      const result = await client.getUser(vars);
      data.value = result;
      loading.value = false;

      if (result.__typename === "User") {
        // Track deferred field states
        for (const [key, deferred] of Object.entries(getDeferredFields(result))) {
          deferredFields[key] = { loading: true, value: null };
          deferred.then((value) => {
            deferredFields[key] = { loading: false, value };
          });
        }
      }
    },
    { immediate: true }
  );

  return { data, loading, deferredFields };
}

/**
 * Mutation composable with optimistic updates
 */
export function useCreatePost(): UseMutationResult<CreatePostResult, CreatePostInput> {
  const loading = ref(false);
  const error = shallowRef<NetworkError | null>(null);
  const data = shallowRef<CreatePostResult | null>(null);

  async function mutate(
    input: CreatePostInput,
    options?: MutationOptions
  ): Promise<CreatePostResult> {
    loading.value = true;
    error.value = null;
    try {
      const result = await client.createPost({ input }, options);
      data.value = result;
      return result;
    } catch (e: unknown) {
      const networkError = toNetworkError(e);
      error.value = networkError;
      throw networkError;
    } finally {
      loading.value = false;
    }
  }

  return { mutate, loading, error, data };
}
```

### 12.2 Type-Safe Composable Types

```typescript
// Composable return types with full inference
interface UseQueryResult<TData> {
  data: Ref<TData | null>;
  loading: Ref<boolean>;
  error: Ref<NetworkError | null>;
  refetch: () => Promise<void>;
}

interface UseStreamingQueryResult<TData> {
  data: Ref<TData | null>;
  loading: Ref<boolean>;
  deferredFields: Record<string, { loading: boolean; value: unknown }>;
}

interface UseMutationResult<TData, TInput> {
  mutate: (input: TInput, options?: MutationOptions) => Promise<TData>;
  loading: Ref<boolean>;
  error: Ref<NetworkError | null>;
  data: Ref<TData | null>;
}

interface UseSubscriptionResult<TData> {
  data: Ref<TData | null>;
  state: Ref<"connecting" | "connected" | "disconnected" | "disposed">;
  error: Ref<Error | null>;
  unsubscribe: () => void;
}
```

### 12.3 Composable Usage Examples

```vue
<script setup lang="ts">
import { computed } from "vue";
import { useGetUser, useCreatePost } from "@/generated/composables";
import type { UserId } from "@/generated/types";

const props = defineProps<{ userId: UserId }>();

// Query with reactive variables
const { data: userResult, loading, error, refetch } = useGetUser(
  computed(() => ({ id: props.userId }))
);

// Type-safe computed properties
const user = computed(() => {
  if (userResult.value?.__typename === "User") {
    return userResult.value;
  }
  return null;
});

const errorMessage = computed(() => {
  if (userResult.value?.__typename === "NotFoundError") {
    return userResult.value.message;
  }
  if (userResult.value?.__typename === "UnauthorizedError") {
    return "Please log in to view this profile";
  }
  return null;
});

// Mutation with full type safety
const { mutate: createPost, loading: creating } = useCreatePost();

async function handleCreatePost(title: string, content: string) {
  const result = await createPost({
    title,
    content,
    authorId: props.userId,
  });

  switch (result.__typename) {
    case "Post":
      showToast(`Created: ${result.title}`);
      break;
    case "ValidationError":
      showError(`${result.field}: ${result.message}`);
      break;
  }
}
</script>

<template>
  <div v-if="loading" class="skeleton" />
  <div v-else-if="error" class="error">{{ error.message }}</div>
  <div v-else-if="errorMessage" class="error">{{ errorMessage }}</div>
  <div v-else-if="user" class="profile">
    <h1>{{ user.name }}</h1>
    <p>{{ user.email }}</p>
  </div>
</template>
```

### 12.4 Streaming with Suspense

```vue
<script setup lang="ts">
import { useGetUserStreaming } from "@/generated/composables";

const props = defineProps<{ userId: UserId }>();

const { data, loading, deferredFields } = useGetUserStreaming(
  computed(() => ({ id: props.userId }))
);

// Access deferred fields with loading states
const bio = computed(() => deferredFields.bio);
const posts = computed(() => deferredFields.posts);
</script>

<template>
  <Suspense>
    <template #default>
      <div v-if="data?.__typename === 'User'" class="profile">
        <h1>{{ data.name }}</h1>

        <!-- Deferred bio with loading state -->
        <div v-if="bio?.loading" class="skeleton" />
        <p v-else>{{ bio?.value }}</p>

        <!-- Deferred posts with loading state -->
        <div v-if="posts?.loading" class="skeleton-list" />
        <ul v-else>
          <li v-for="post in posts?.value" :key="post.id">
            {{ post.title }}
          </li>
        </ul>
      </div>
    </template>
    <template #fallback>
      <div class="loading">Loading profile...</div>
    </template>
  </Suspense>
</template>
```

### 12.5 Subscription Composable

```typescript
// Generated subscription composable
export function usePostCreated(
  variables: MaybeRef<{ authorId?: UserId }>
): UseSubscriptionResult<Post> {
  const data = ref<Post | null>(null);
  const state = ref<SubscriptionState>("connecting");
  const error = ref<Error | null>(null);
  let subscription: Subscription<Post> | null = null;

  onMounted(async () => {
    subscription = client.subscribe.postCreated(unref(variables));

    subscription.onReconnect(() => {
      state.value = "connected";
    });

    subscription.onDisconnect((reason) => {
      state.value = "disconnected";
      if (!reason.retryable) {
        error.value = new Error(reason.message);
      }
    });

    state.value = "connected";

    for await (const post of subscription) {
      data.value = post;
    }
  });

  onUnmounted(async () => {
    state.value = "disposed";
    await subscription?.[Symbol.asyncDispose]();
  });

  return {
    data,
    state,
    error,
    unsubscribe: () => subscription?.abort(),
  };
}
```

### 12.6 Subscription Usage

```vue
<script setup lang="ts">
import { ref } from "vue";
import { usePostCreated } from "@/generated/composables";

const posts = ref<Post[]>([]);

const { data: newPost, state } = usePostCreated({ authorId: undefined });

// Reactively add new posts
watch(newPost, (post) => {
  if (post) {
    posts.value.unshift(post);
  }
});
</script>

<template>
  <div class="live-feed">
    <div class="status" :class="state">
      <span v-if="state === 'connecting'">Connecting...</span>
      <span v-else-if="state === 'connected'" class="connected">Live</span>
      <span v-else-if="state === 'disconnected'">Reconnecting...</span>
    </div>

    <TransitionGroup name="list" tag="ul">
      <li v-for="post in posts" :key="post.id">
        {{ post.title }}
      </li>
    </TransitionGroup>
  </div>
</template>

<style scoped>
.list-enter-active,
.list-leave-active {
  transition: all 0.3s ease;
}
.list-enter-from {
  opacity: 0;
  transform: translateX(-30px);
}
</style>
```

### 12.7 Provide/Inject Pattern

```typescript
// Provide client at app root
import { provide, inject, type InjectionKey } from "vue";
import { createClient, type Client } from "@/generated/client";

const ClientKey: InjectionKey<Client> = Symbol("BetterGraphQLClient");

// In App.vue
const client = createClient({
  endpoint: import.meta.env.VITE_GRAPHQL_ENDPOINT,
})
  .intercept("UnauthorizedError", () => {
    router.push("/login");
    return true;
  });

provide(ClientKey, client);

// In any component
function useClient(): Client {
  const client = inject(ClientKey);
  if (!client) {
    throw new Error("Client not provided");
  }
  return client;
}
```

## 12. Type Safety Guarantees

### 12.1 Compile-Time Checks

The generated client provides compile-time guarantees:

1. **Variable type safety** - Wrong variable types are caught at compile time
2. **Response type safety** - All possible response types are known
3. **Newtype safety** - Different ID types cannot be mixed
4. **Exhaustive switch** - All union cases must be handled in switch statements

### 12.2 Runtime Validation

Optional runtime validation for defense in depth:

```typescript
const client = createClient({
  endpoint: "https://api.example.com/graphql",
  validation: {
    // Validate response matches expected types
    validateResponse: true,
    // Validate newtypes at runtime
    validateNewtypes: true,
    // Custom validators
    validators: {
      EmailAddress: (value) => /^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(value),
    },
  },
});
```

## 13. Browser DevTools Extension

The bgql client SDK includes a browser extension for debugging GraphQL operations in real-time.

### 13.1 Installation

```bash
# Build the extension
cd npm/client/devtools
bun run build

# Chrome: Load unpacked extension from dist/chrome
# Firefox: Load temporary add-on from dist/firefox/manifest.json
```

### 13.2 Client Integration

```typescript
import { createClient } from "@bgql/client";
import { devtools } from "@bgql/client/devtools";

const client = createClient({
  endpoint: "/graphql",
  plugins: [
    devtools({
      name: "My App",                    // Display name in DevTools
      enabled: import.meta.env.DEV,      // Only enable in development
      logToConsole: false,               // Don't duplicate logs
      maxEntries: 100,                   // Limit stored operations
    }),
  ],
});
```

### 13.3 DevTools Panels

#### Query Inspector

Real-time view of all GraphQL operations:

```
┌─────────────────────────────────────────────────────────────┐
│ bgql DevTools                                   [Queries ▼] │
├─────────────────────────────────────────────────────────────┤
│ ● GetUser              query     150ms   ✓ Success          │
│   ├─ Variables: { id: "user_1" }                            │
│   ├─ Trace ID: abc123-def456                                │
│   └─ Streaming:                                             │
│       ├─ @defer: stats (45ms)                               │
│       └─ @stream: posts (3 items, 89ms)                     │
├─────────────────────────────────────────────────────────────┤
│ ● CreatePost           mutation  230ms   ✓ Success          │
│   └─ Variables: { input: { title: "...", ... } }            │
├─────────────────────────────────────────────────────────────┤
│ ○ FeedSubscription     subscription  ⏳ Active              │
│   └─ Events: 12 received                                    │
└─────────────────────────────────────────────────────────────┘
```

- Filter by operation type (query/mutation/subscription)
- Search by operation name or variables
- View full request/response payloads
- Copy operations as cURL commands

#### Streaming Visualizer

Timeline view for `@defer` and `@stream` operations:

```
┌─────────────────────────────────────────────────────────────┐
│ Query: GetDashboard                                         │
├─────────────────────────────────────────────────────────────┤
│ Timeline                                                    │
│ ├─ 0ms   ────●──── Initial response                         │
│ │              user.id, user.name, user.avatarUrl           │
│ │                                                           │
│ ├─ 45ms  ────────●──── @defer(label: "stats")               │
│ │                   postsCount: 42, followersCount: 128     │
│ │                                                           │
│ ├─ 80ms  ────────────●── @stream(label: "feed") [0-2]       │
│ │                        3 items received                   │
│ │                                                           │
│ ├─ 120ms ─────────────●── @stream(label: "feed") [3-5]      │
│ │                         3 items received                  │
│ │                                                           │
│ └─ 160ms ──────────────●── @stream(label: "feed") [6-9]     │
│                            4 items received (final)         │
│                                                             │
│ Total: 9 stream chunks, 1 defer payload                     │
│                                                             │
│ [▶ Replay] [📋 Copy Response] [📥 Export HAR]               │
└─────────────────────────────────────────────────────────────┘
```

- Visual timeline of all streaming chunks
- Inspect each chunk's payload
- Replay streaming sequence
- Export as HAR for sharing

#### Cache Explorer

Inspect the normalized cache contents:

```
┌─────────────────────────────────────────────────────────────┐
│ Normalized Cache                          [🔍 Search types] │
├─────────────────────────────────────────────────────────────┤
│ ▼ User (3 entries)                                          │
│   ├─ User:user_1                                            │
│   │   {                                                     │
│   │     "id": "user_1",                                     │
│   │     "name": "Alice",                                    │
│   │     "email": "alice@example.com",                       │
│   │     "__typename": "User"                                │
│   │   }                                                     │
│   ├─ User:user_2                                            │
│   └─ User:user_3                                            │
│                                                             │
│ ▼ Post (12 entries)                                         │
│   ├─ Post:post_1                                            │
│   │   { "id": "post_1", "title": "Hello", ... }             │
│   └─ ... (11 more)                                          │
│                                                             │
│ ▶ Query (2 entries)                                         │
│ ▶ __META__ (1 entry)                                        │
│                                                             │
│ Cache size: 45.2 KB | Entries: 18                           │
│ [🗑️ Clear Cache] [📋 Export JSON]                          │
└─────────────────────────────────────────────────────────────┘
```

- Search and filter by type
- View entity relationships
- Track cache updates in real-time
- Clear cache for testing

#### Network Timeline

Waterfall visualization of network requests:

```
┌─────────────────────────────────────────────────────────────┐
│ Network                                        [Clear All]  │
├─────────────────────────────────────────────────────────────┤
│ Request          Status   Time      Size                    │
├─────────────────────────────────────────────────────────────┤
│ GetUser          200 OK   150ms     2.4 KB                  │
│ ├────────█████████████████───────────────────────────────── │
│ │        ↑ Initial        ↑ @defer                          │
│                                                             │
│ GetDashboard     200 OK   320ms     12.1 KB                 │
│ ├───█████─────█████─────█████─────█████───────────────────  │
│ │   ↑          ↑          ↑         ↑ @stream chunks        │
│                                                             │
│ CreatePost       200 OK   89ms      0.8 KB                  │
│ ├────████████────────────────────────────────────────────── │
└─────────────────────────────────────────────────────────────┘
```

- Visual waterfall of streaming chunks
- Request/response size breakdown
- Headers and timing details
- Binary stream progress

#### Schema Browser

Navigate and search the GraphQL schema:

```
┌─────────────────────────────────────────────────────────────┐
│ Schema                                    [🔍 Search types] │
├─────────────────────────────────────────────────────────────┤
│ ▼ Types                                                     │
│   ├─ User                                                   │
│   │   """A registered user in the system"""                 │
│   │   type User {                                           │
│   │     id: UserId!                                         │
│   │     name: String!                                       │
│   │     email: String!                                      │
│   │     posts(first: Int): [Post!]!                         │
│   │   }                                                     │
│   ├─ Post                                                   │
│   └─ ...                                                    │
│                                                             │
│ ▼ Queries                                                   │
│   ├─ user(id: UserId!): UserResult!                         │
│   └─ users(first: Int, after: String): UserConnection!      │
│                                                             │
│ ▶ Mutations                                                 │
│ ▶ Subscriptions                                             │
│ ▶ Directives                                                │
└─────────────────────────────────────────────────────────────┘
```

- Full schema documentation
- Click-through type navigation
- Search by type/field name
- Copy SDL definitions

### 13.4 DevTools Protocol

The devtools extension communicates via a simple message protocol:

```typescript
// Message types
type DevToolsMessage =
  | { type: "operation:start"; id: string; operation: OperationInfo }
  | { type: "operation:data"; id: string; data: unknown }
  | { type: "operation:defer"; id: string; label: string; data: unknown }
  | { type: "operation:stream"; id: string; label: string; items: unknown[] }
  | { type: "operation:error"; id: string; error: GraphQLError[] }
  | { type: "operation:complete"; id: string; duration: number }
  | { type: "cache:update"; entries: CacheEntry[] }
  | { type: "cache:evict"; keys: string[] };

// Hook into client events
client.on("operation:start", (op) => {
  window.postMessage({ type: "bgql:event", payload: op }, "*");
});
```

## 14. Summary

Better GraphQL client SDK design ensures:

| Feature | Benefit |
|---------|---------|
| Typed unions for results | No unexpected nulls or exceptions |
| No throw-based errors | Predictable control flow |
| Type-safe interceptors | Global error handling with narrowed types |
| Branded newtypes | Prevent ID type confusion |
| Partial Promise with `await` | Natural deferred field access |
| Native AbortController | Standard cancellation API for all operations |
| `using` disposable | Automatic subscription cleanup |
| Switch-based patterns | Native exhaustive case handling |
| Vue composables | Fully reactive queries, mutations, subscriptions |
| Browser DevTools | Visual debugging for streaming and cache |
| Structured logging | Distributed trace context for observability |
