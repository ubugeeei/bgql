# Better GraphQL Specification - Introspection

## 1. Overview

Better GraphQL provides a comprehensive introspection system that allows clients to query the schema at runtime. This extends GraphQL's introspection with additional metadata for validation rules, error types, and server-side fragments.

## 2. Introspection Types

### 2.1 __Schema

```graphql
type __Schema {
  """The description of the schema"""
  description: Option<String>

  """All types in the schema"""
  types: List<__Type>

  """The root query type"""
  queryType: __Type

  """The root mutation type"""
  mutationType: Option<__Type>

  """The root subscription type"""
  subscriptionType: Option<__Type>

  """All directives in the schema"""
  directives: List<__Directive>

  """Server-side fragments (Better GraphQL extension)"""
  serverFragments: List<__ServerFragment>

  """CORS configuration (Better GraphQL extension)"""
  corsConfig: Option<__CorsConfig>
}
```

### 2.2 __Type

```graphql
type __Type {
  kind: __TypeKind
  name: Option<String>
  description: Option<String>

  # For OBJECT and INTERFACE
  fields(includeDeprecated: Boolean = false): Option<List<__Field>>
  interfaces: Option<List<__Type>>

  # For INTERFACE and UNION
  possibleTypes: Option<List<__Type>>

  # For ENUM
  enumValues(includeDeprecated: Boolean = false): Option<List<__EnumValue>>

  # For INPUT_OBJECT
  inputFields(includeDeprecated: Boolean = false): Option<List<__InputValue>>

  # For NON_NULL and LIST
  ofType: Option<__Type>

  # Better GraphQL extensions
  """Whether this type implements Error interface"""
  isErrorType: Boolean

  """Validation rules for this type (for input types)"""
  validationRules: Option<List<__ValidationRule>>
}

enum __TypeKind {
  SCALAR
  OBJECT
  INTERFACE
  UNION
  ENUM
  INPUT_OBJECT
  LIST
  NON_NULL
  INPUT_UNION  # Better GraphQL extension
}
```

### 2.3 __Field

```graphql
type __Field {
  name: String
  description: Option<String>
  args(includeDeprecated: Boolean = false): List<__InputValue>
  type: __Type
  isDeprecated: Boolean
  deprecationReason: Option<String>

  # Better GraphQL extensions
  """Directives applied to this field"""
  appliedDirectives: List<__AppliedDirective>

  """Whether this field requires authentication"""
  requiresAuth: Boolean

  """Required roles for this field"""
  requiredRoles: Option<List<String>>

  """Cache configuration"""
  cacheConfig: Option<__CacheConfig>
}
```

### 2.4 __InputValue

```graphql
type __InputValue {
  name: String
  description: Option<String>
  type: __Type
  defaultValue: Option<String>
  isDeprecated: Boolean
  deprecationReason: Option<String>

  # Better GraphQL extensions
  """Validation rules for this input"""
  validationRules: List<__ValidationRule>
}
```

### 2.5 __EnumValue

```graphql
type __EnumValue {
  name: String
  description: Option<String>
  isDeprecated: Boolean
  deprecationReason: Option<String>
}
```

### 2.6 __Directive

```graphql
type __Directive {
  name: String
  description: Option<String>
  locations: List<__DirectiveLocation>
  args: List<__InputValue>
  isRepeatable: Boolean
}

enum __DirectiveLocation {
  QUERY
  MUTATION
  SUBSCRIPTION
  FIELD
  FRAGMENT_DEFINITION
  FRAGMENT_SPREAD
  INLINE_FRAGMENT
  VARIABLE_DEFINITION
  SCHEMA
  SCALAR
  OBJECT
  FIELD_DEFINITION
  ARGUMENT_DEFINITION
  INTERFACE
  UNION
  ENUM
  ENUM_VALUE
  INPUT_OBJECT
  INPUT_FIELD_DEFINITION
}
```

## 3. Better GraphQL Extensions

### 3.1 __ValidationRule

```graphql
type __ValidationRule {
  """Rule name (e.g., 'minLength', 'email', 'pattern')"""
  name: String

  """Rule parameters"""
  params: Option<JSON>

  """Error message when validation fails"""
  message: Option<String>
}
```

### 3.2 __AppliedDirective

```graphql
type __AppliedDirective {
  """Directive name"""
  name: String

  """Applied arguments"""
  args: JSON
}
```

### 3.3 __ServerFragment

```graphql
type __ServerFragment {
  """Fragment name"""
  name: String

  """Target type"""
  onType: __Type

  """Fragment description"""
  description: Option<String>

  """Fragment version"""
  version: Option<String>

  """Whether this fragment is deprecated"""
  isDeprecated: Boolean

  """Deprecation reason"""
  deprecationReason: Option<String>

  """Fields in this fragment"""
  fields: List<__Field>
}
```

### 3.4 __CacheConfig

```graphql
type __CacheConfig {
  """Max age in seconds"""
  maxAge: Int

  """Cache scope"""
  scope: CacheScope

  """Stale-while-revalidate duration"""
  swr: Option<Int>

  """Headers to vary on"""
  vary: List<String>
}

enum CacheScope {
  Public
  Private
}
```

### 3.5 __CorsConfig

```graphql
type __CorsConfig {
  """Allowed origins"""
  origins: List<String>

  """Allowed methods"""
  methods: List<String>

  """Allowed headers"""
  allowHeaders: List<String>

  """Exposed headers"""
  exposeHeaders: List<String>

  """Allow credentials"""
  credentials: Boolean

  """Max age for preflight cache"""
  maxAge: Int
}
```

## 4. Introspection Queries

### 4.1 Full Schema Query

```graphql
query IntrospectionQuery {
  __schema {
    queryType { name }
    mutationType { name }
    subscriptionType { name }
    types {
      ...FullType
    }
    directives {
      name
      description
      locations
      args {
        ...InputValue
      }
    }
  }
}

fragment FullType on __Type {
  kind
  name
  description
  fields(includeDeprecated: true) {
    name
    description
    args {
      ...InputValue
    }
    type {
      ...TypeRef
    }
    isDeprecated
    deprecationReason
    requiresAuth
    requiredRoles
  }
  inputFields {
    ...InputValue
  }
  interfaces {
    ...TypeRef
  }
  enumValues(includeDeprecated: true) {
    name
    description
    isDeprecated
    deprecationReason
  }
  possibleTypes {
    ...TypeRef
  }
  isErrorType
  validationRules {
    name
    params
    message
  }
}

fragment InputValue on __InputValue {
  name
  description
  type {
    ...TypeRef
  }
  defaultValue
  validationRules {
    name
    params
    message
  }
}

fragment TypeRef on __Type {
  kind
  name
  ofType {
    kind
    name
    ofType {
      kind
      name
      ofType {
        kind
        name
        ofType {
          kind
          name
          ofType {
            kind
            name
            ofType {
              kind
              name
              ofType {
                kind
                name
              }
            }
          }
        }
      }
    }
  }
}
```

### 4.2 Type Query

```graphql
query TypeQuery($name: String) {
  __type(name: $name) {
    kind
    name
    description
    fields {
      name
      type {
        kind
        name
        ofType {
          kind
          name
        }
      }
      args {
        name
        type {
          kind
          name
        }
        defaultValue
        validationRules {
          name
          params
        }
      }
    }
    isErrorType
  }
}
```

### 4.3 Error Types Query

```graphql
query ErrorTypesQuery {
  __schema {
    types {
      name
      kind
      isErrorType
      fields {
        name
        type {
          name
        }
      }
    }
  }
}
```

### 4.4 Server Fragments Query

```graphql
query ServerFragmentsQuery {
  __schema {
    serverFragments {
      name
      description
      version
      onType {
        name
      }
      fields {
        name
        type {
          name
        }
      }
      isDeprecated
      deprecationReason
    }
  }
}
```

### 4.5 Validation Rules Query

```graphql
query ValidationRulesQuery($typeName: String) {
  __type(name: $typeName) {
    name
    inputFields {
      name
      type {
        name
      }
      validationRules {
        name
        params
        message
      }
    }
  }
}
```

### 4.6 CORS Config Query

```graphql
query CorsConfigQuery {
  __schema {
    corsConfig {
      origins
      methods
      allowHeaders
      exposeHeaders
      credentials
      maxAge
    }
  }
}
```

## 5. Using Introspection

### 5.1 Client Code Generation

Introspection enables automatic client code generation:

```typescript
// Generated from introspection
interface User {
  id: string;
  name: string;
  email: string;
  avatarUrl?: string;
}

type UserResult =
  | { __typename: 'User' } & User
  | { __typename: 'NotFoundError'; message: string; resourceId: string }
  | { __typename: 'UnauthorizedError'; message: string };
```

### 5.2 Schema Validation

Introspection enables schema comparison and validation:

```typescript
const diff = compareSchemas(oldSchema, newSchema);

if (diff.breakingChanges.length > 0) {
  console.error('Breaking changes detected:', diff.breakingChanges);
}
```

### 5.3 IDE Integration

Introspection powers IDE features:

- Autocomplete for fields and arguments
- Type information on hover
- Validation error highlighting
- Navigation to type definitions

### 5.4 Documentation Generation

Introspection enables automatic documentation:

```markdown
## User

Represents a user in the system.

### Fields

| Field | Type | Description |
|-------|------|-------------|
| id | ID! | Unique identifier |
| name | String | Display name |
| email | String | Email address |
```

## 6. Disabling Introspection

For production environments, introspection can be disabled:

```yaml
# better-graphql.config.yaml
introspection:
  enabled: false
  # Or conditionally
  enabledEnvironments: ["development", "staging"]
```

Or per-request:
```graphql
schema @introspection(enabled: false) {
  query: Query
}
```

## 7. Introspection Security

### 7.1 Field Filtering

Hide sensitive fields from introspection:

```graphql
type User {
  id: ID
  name: String
  secretKey: String @internal  # Hidden from introspection
}
```

### 7.2 Type Filtering

Hide internal types:

```graphql
type InternalAuditLog @internal {
  action: String
  userId: ID
}
```

### 7.3 Depth Limiting

Limit introspection query depth:

```yaml
introspection:
  maxDepth: 10
```
