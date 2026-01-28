# Better GraphQL Specification - Query Language

## 1. Overview

The Better GraphQL query language allows clients to precisely specify the data they need from the API. It is largely compatible with GraphQL query syntax while adding new features.

## 2. Operations

### 2.1 Operation Types

Better GraphQL supports three operation types:

| Operation | Description |
|-----------|-------------|
| `query` | Read-only data fetching |
| `mutation` | Data modification |
| `subscription` | Real-time data streaming |

### 2.2 Operation Definition

```graphql
# Named operation with variables
query GetUser($id: ID, $includeDetails: Boolean = false) {
  user(id: $id) {
    id
    name
    ... @include(if: $includeDetails) {
      email
      bio
    }
  }
}

# Shorthand query (when there's only one query without variables)
{
  users {
    id
    name
  }
}

# Mutation
mutation CreateUser($input: CreateUserInput) {
  createUser(input: $input) {
    ... on User {
      id
      name
    }
    ... on ValidationError {
      message
      field
    }
  }
}

# Subscription
subscription OnUserCreated {
  userCreated {
    id
    name
  }
}
```

## 3. Fields

### 3.1 Field Selection

```graphql
query {
  user(id: "123") {
    id          # Scalar field
    name        # Scalar field
    profile {   # Object field
      bio
      website
    }
    posts {     # List field
      id
      title
    }
  }
}
```

### 3.2 Field Arguments

```graphql
query {
  # Simple argument
  user(id: "123") {
    name
  }

  # Multiple arguments
  users(first: 10, orderBy: CREATED_AT_DESC) {
    id
    name
  }

  # Input object argument
  searchUsers(filter: { role: ADMIN, isActive: true }) {
    id
    name
  }
}
```

### 3.3 Field Aliases

Aliases allow fetching the same field with different arguments.

```graphql
query {
  # Use aliases to fetch the same field multiple times
  admin: user(id: "1") {
    name
    role
  }
  moderator: user(id: "2") {
    name
    role
  }

  # Alias for computed fields
  recentPosts: posts(first: 5, orderBy: CREATED_AT_DESC) {
    title
  }
  popularPosts: posts(first: 5, orderBy: LIKES_DESC) {
    title
  }
}
```

## 4. Variables

### 4.1 Variable Definition

```graphql
query GetUsers(
  $first: Int = 10,           # With default value
  $after: Option<String>,     # Nullable
  $filter: UserFilter,        # Required (non-null)
  $includeInactive: Boolean = false
) {
  users(first: $first, after: $after, filter: $filter) {
    id
    name
    isActive @include(if: $includeInactive)
  }
}
```

### 4.2 Variable Values (JSON)

```json
{
  "first": 20,
  "after": "cursor_abc123",
  "filter": {
    "role": "ADMIN"
  }
}
```

### 4.3 Variable Types

Variables can be any input type:

- Scalar types: `String`, `Int`, `Float`, `Boolean`, `ID`
- Enum types
- Input object types
- Input union types
- List types

## 5. Fragments

### 5.1 Fragment Definition (Client-side)

```graphql
fragment UserBasicInfo on User {
  id
  name
  avatarUrl
}

fragment UserFullInfo on User {
  ...UserBasicInfo
  email
  bio
  createdAt
}

query GetUsers {
  users {
    ...UserBasicInfo
  }
  currentUser {
    ...UserFullInfo
  }
}
```

### 5.2 Inline Fragments

```graphql
query GetSearchResults($query: String) {
  search(query: $query) {
    ... on User {
      id
      name
      email
    }
    ... on Post {
      id
      title
      author {
        name
      }
    }
    ... on Comment {
      id
      content
      post {
        title
      }
    }
  }
}
```

### 5.3 Server-side Fragments

Better GraphQL allows using server-managed fragments.

```graphql
# Using @use directive
query GetUser($id: ID) {
  user(id: $id) {
    ... on User @use(fragment: "UserDetail") {
      # Fields from UserDetail fragment are included
    }
  }
}

# Shorthand syntax
query GetUser($id: ID) {
  user(id: $id) {
    ... on User {
      ...@UserDetail
    }
  }
}

# With versioned fragment
query GetUser($id: ID) {
  user(id: $id) {
    ... on User @use(fragment: "UserDetail", version: "2024-01") {
      # Fields from specific version
    }
  }
}
```

## 6. Directives in Queries

### 6.1 @include and @skip

```graphql
query GetUser($id: ID, $withPosts: Boolean, $hideSensitive: Boolean) {
  user(id: $id) {
    id
    name
    email @skip(if: $hideSensitive)
    posts @include(if: $withPosts) {
      id
      title
    }
  }
}
```

### 6.2 @defer

```graphql
query GetUser($id: ID) {
  user(id: $id) {
    id
    name
    ... @defer(label: "profile") {
      bio
      avatarUrl
      followers {
        id
        name
      }
    }
    ... @defer(label: "stats", priority: 2) {
      postsCount
      followersCount
      likesCount
    }
  }
}
```

### 6.3 @stream

```graphql
query GetTimeline {
  timeline @stream(initialCount: 10) {
    id
    content
    author {
      name
    }
    ... @defer {
      comments {
        id
        content
      }
    }
  }
}
```

## 7. Type Conditions

### 7.1 Union Type Handling

```graphql
query GetUser($id: ID) {
  user(id: $id) {
    __typename
    ... on UserPayload {
      user {
        id
        name
      }
    }
    ... on NotFoundError {
      message
      resourceId
    }
    ... on UnauthorizedError {
      message
      requiredPermission
    }
  }
}
```

### 7.2 Interface Type Handling

```graphql
query GetNodes($ids: List<ID>) {
  nodes(ids: $ids) {
    __typename
    id  # Common interface field
    ... on User {
      name
      email
    }
    ... on Post {
      title
      content
    }
    ... on Comment {
      body
      author {
        name
      }
    }
  }
}
```

## 8. Nested Selections

### 8.1 Deep Nesting

```graphql
query GetUserWithRelations($id: ID) {
  user(id: $id) {
    id
    name
    posts(first: 5) {
      id
      title
      comments(first: 3) {
        id
        content
        author {
          id
          name
        }
        replies(first: 2) {
          id
          content
        }
      }
    }
  }
}
```

### 8.2 Connection Pattern (Pagination)

```graphql
query GetUserPosts($userId: ID, $first: Int, $after: String) {
  user(id: $userId) {
    posts(first: $first, after: $after) {
      edges {
        cursor
        node {
          id
          title
          createdAt
        }
      }
      pageInfo {
        hasNextPage
        hasPreviousPage
        startCursor
        endCursor
      }
      totalCount
    }
  }
}
```

## 9. Introspection Queries

### 9.1 Type Introspection

```graphql
query TypeInfo {
  __type(name: "User") {
    name
    kind
    description
    fields {
      name
      type {
        name
        kind
      }
      args {
        name
        type {
          name
        }
        defaultValue
      }
    }
  }
}
```

### 9.2 Schema Introspection

```graphql
query SchemaInfo {
  __schema {
    queryType {
      name
    }
    mutationType {
      name
    }
    subscriptionType {
      name
    }
    types {
      name
      kind
    }
    directives {
      name
      locations
      args {
        name
        type {
          name
        }
      }
    }
  }
}
```

## 10. Query Validation Rules

### 10.1 Field Selection Validation

- Fields MUST be defined on the selected type
- Leaf fields (scalars, enums) MUST NOT have sub-selections
- Object fields MUST have sub-selections

### 10.2 Argument Validation

- Required arguments MUST be provided
- Argument types MUST match
- Unknown arguments are NOT allowed

### 10.3 Fragment Validation

- Fragment type conditions MUST match the target type
- Fragment cycles are NOT allowed
- Fragments MUST be used at least once

### 10.4 Variable Validation

- Variables MUST be defined
- Variable types MUST be compatible with argument types
- Variables MUST be used at least once

## 11. Query Complexity

### 11.1 Depth Limiting

Servers MAY limit query depth:

```graphql
# This might be rejected if depth limit is 4
query TooDeep {
  user {           # Depth 1
    posts {        # Depth 2
      comments {   # Depth 3
        author {   # Depth 4
          posts {  # Depth 5 - Exceeds limit
            title
          }
        }
      }
    }
  }
}
```

### 11.2 Complexity Calculation

Servers MAY calculate and limit query complexity:

```graphql
# @cost directive for complexity calculation (schema-side)
type Query {
  users(first: Int): List<User> @cost(multiplier: "first", base: 1)
}

type User {
  posts(first: Int): List<Post> @cost(multiplier: "first", base: 2)
}
```
