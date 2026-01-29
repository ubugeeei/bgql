# Types

Better GraphQL provides a rich type system with explicit nullability, generics, and Rust-inspired features.

## Scalar Types

### Built-in Scalars

| Type | Description | TypeScript |
|------|-------------|------------|
| `Int` | 32-bit signed integer | `number` |
| `Float` | Double-precision float | `number` |
| `String` | UTF-8 string | `string` |
| `Boolean` | true/false | `boolean` |
| `ID` | Unique identifier | `string` |

### Common Scalars

| Type | Description | Example |
|------|-------------|---------|
| `DateTime` | ISO 8601 date-time | `2024-01-15T09:30:00Z` |
| `Date` | ISO 8601 date | `2024-01-15` |
| `Time` | ISO 8601 time | `09:30:00` |
| `JSON` | Arbitrary JSON | `{ "key": "value" }` |
| `UUID` | UUID v4 | `550e8400-e29b-41d4-a716-446655440000` |

### Custom Scalars

```graphql
scalar Email @specifiedBy(url: "https://html.spec.whatwg.org/#valid-e-mail-address")
scalar URL @specifiedBy(url: "https://url.spec.whatwg.org/")
scalar BigInt
scalar Decimal
```

## Object Types

Object types define the shape of your data:

```graphql
type User {
  id: ID
  name: String
  email: String
  age: Int
  isActive: Boolean
}
```

### With Documentation

```graphql
"""
Represents a user in the system.
Users can have multiple posts and belong to organizations.
"""
type User {
  """Unique identifier for the user"""
  id: ID

  """Display name (1-100 characters)"""
  name: String

  """Email address, unique across all users"""
  email: String
}
```

### With Directives

```graphql
type User {
  id: ID
  name: String
  email: String @requireAuth
  password: String @internal  # Not exposed in queries
  createdAt: DateTime @deprecated(reason: "Use timestamps.created")
}
```

## Nullability

Unlike traditional GraphQL where fields are nullable by default, Better GraphQL uses explicit nullability:

### Non-Null (Default)

```graphql
type User {
  id: ID        # Required - never null
  name: String  # Required - never null
}
```

### Optional with `Option<T>`

```graphql
type User {
  id: ID
  name: String
  bio: Option<String>           # Can be null
  avatarUrl: Option<String>     # Can be null
  deletedAt: Option<DateTime>   # Can be null
}
```

### TypeScript Mapping

```typescript
// bgql
type User {
  id: ID
  name: String
  bio: Option<String>
}

// Generated TypeScript
interface User {
  id: string        // Required
  name: string      // Required
  bio: string | null  // Nullable
}
```

## List Types

### Non-Empty List

```graphql
type User {
  tags: List<String>  # List that can be empty
}
```

### List with Optional Items

```graphql
type SearchResult {
  # List of optional items (some might be null)
  results: List<Option<Item>>
}
```

### Optional List

```graphql
type User {
  # The list itself is optional
  nickname: Option<List<String>>
}
```

## Field Arguments

Fields can have arguments:

```graphql
type Query {
  # Simple argument
  user(id: ID): User

  # Multiple arguments
  users(first: Int, after: String): List<User>

  # Default values
  posts(
    limit: Int = 10,
    offset: Int = 0,
    status: PostStatus = PUBLISHED
  ): List<Post>

  # Optional arguments
  search(
    query: String,
    filter: Option<SearchFilter>
  ): List<SearchResult>
}
```

### Argument Documentation

```graphql
type Query {
  users(
    """Maximum number of users to return"""
    first: Int = 10,

    """Cursor for pagination"""
    after: Option<String>,

    """Filter by role"""
    role: Option<UserRole>
  ): Connection<User>
}
```

## Visibility

Control type visibility with `pub`:

```graphql
# Public type - can be imported by other modules
pub type User {
  id: ID
  name: String
}

# Private type - only visible in this module
type InternalConfig {
  secret: String
}
```

## Type Extension

Extend existing types:

```graphql
# Base definition
type User {
  id: ID
  name: String
}

# Extension (can be in another file)
extend type User {
  email: String
  posts: List<Post>
}

# Result: User has id, name, email, and posts
```

## Complete Example

```graphql
"""
Core user type representing authenticated users.
"""
pub type User implements Node & Timestamped {
  """Unique identifier (UUID v4)"""
  id: ID

  """Display name, 1-100 characters"""
  name: String

  """Unique email address"""
  email: String @requireAuth

  """Optional biography"""
  bio: Option<String>

  """Profile picture URL"""
  avatarUrl: Option<String>

  """User's role in the system"""
  role: UserRole

  """When the user was created"""
  createdAt: DateTime

  """When the user was last updated"""
  updatedAt: Option<DateTime>

  """User's posts with pagination"""
  posts(
    first: Int = 10,
    after: Option<String>
  ): Connection<Post>

  """Organizations the user belongs to"""
  organizations: List<Organization>
}
```

## Best Practices

### 1. Use Explicit Nullability

```graphql
# ✅ Good: Clear about what can be null
type User {
  id: ID
  name: String
  bio: Option<String>
}

# ❌ Avoid: Unclear nullability (traditional GraphQL style)
type User {
  id: ID!
  name: String!
  bio: String
}
```

### 2. Document Your Types

```graphql
# ✅ Good: Well-documented
"""
Represents a blog post.
Posts can be in draft or published state.
"""
type Post {
  """Unique identifier"""
  id: ID

  """Post title, max 200 characters"""
  title: String
}

# ❌ Avoid: No documentation
type Post {
  id: ID
  title: String
}
```

### 3. Use Semantic Types

```graphql
# ✅ Good: Semantic types
type User {
  id: ID
  email: Email
  website: Option<URL>
  createdAt: DateTime
}

# ❌ Avoid: Generic strings
type User {
  id: String
  email: String
  website: String
  createdAt: String
}
```

## Next Steps

- [Interfaces](/schema/interfaces)
- [Enums and Unions](/schema/enums-unions)
- [Generics](/schema/generics)
