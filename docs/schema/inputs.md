# Input Types

Input types define the shape of data passed to queries and mutations.

## Basic Input

```graphql
input CreateUserInput {
  name: String
  email: String
  password: String
}

type Mutation {
  createUser(input: CreateUserInput): User
}
```

## Optional Fields

Use `Option<T>` for optional input fields:

```graphql
input UpdateUserInput {
  name: Option<String>
  email: Option<String>
  bio: Option<String>
  avatarUrl: Option<String>
}

type Mutation {
  # Only provided fields are updated
  updateUser(id: ID, input: UpdateUserInput): User
}
```

## Default Values

```graphql
input PaginationInput {
  first: Int = 10
  after: Option<String>
  orderBy: SortOrder = DESC
}

input SearchInput {
  query: String
  limit: Int = 20
  offset: Int = 0
  includeArchived: Boolean = false
}
```

## Nested Inputs

```graphql
input AddressInput {
  street: String
  city: String
  state: String
  zipCode: String
  country: String = "US"
}

input CreateUserInput {
  name: String
  email: String
  address: Option<AddressInput>
  billingAddress: Option<AddressInput>
}
```

## Input Validation

Use directives for validation:

```graphql
input CreateUserInput {
  name: String @minLength(1) @maxLength(100)
  email: String @email
  password: String @minLength(8) @pattern(regex: "^(?=.*[A-Za-z])(?=.*\\d).+$")
  age: Option<Int> @min(0) @max(150)
  website: Option<String> @url
}

input CreatePostInput {
  title: String @minLength(1) @maxLength(200)
  content: String @maxLength(50000)
  tags: List<String> @maxLength(10)  # Max 10 tags
}
```

## Input with Lists

```graphql
input CreateOrderInput {
  items: List<OrderItemInput>
  shippingAddress: AddressInput
  couponCodes: Option<List<String>>
}

input OrderItemInput {
  productId: ID
  quantity: Int @min(1)
}
```

## Input Unions (Discriminated Inputs)

```graphql
input union PaymentMethod {
  CreditCard {
    cardNumber: String
    expiry: String
    cvv: String
  }
  BankTransfer {
    accountNumber: String
    routingNumber: String
  }
  PayPal {
    email: String
  }
}

input CheckoutInput {
  cartId: ID
  paymentMethod: PaymentMethod
  billingAddress: AddressInput
}
```

Usage:

```graphql
mutation Checkout {
  checkout(input: {
    cartId: "cart_123"
    paymentMethod: {
      CreditCard: {
        cardNumber: "4111111111111111"
        expiry: "12/25"
        cvv: "123"
      }
    }
    billingAddress: {
      street: "123 Main St"
      city: "San Francisco"
      state: "CA"
      zipCode: "94105"
    }
  }) {
    orderId
    status
  }
}
```

## Input Enums (Rust-style)

```graphql
input enum FilterCondition {
  Equals { value: String }
  Contains { value: String }
  GreaterThan { value: Int }
  LessThan { value: Int }
  Between { min: Int, max: Int }
  IsNull
  IsNotNull
}

input FieldFilter {
  field: String
  condition: FilterCondition
}

input SearchInput {
  filters: List<FieldFilter>
  orderBy: Option<String>
}
```

## Reusable Input Patterns

### Pagination

```graphql
input PaginationInput {
  first: Option<Int>
  after: Option<String>
  last: Option<Int>
  before: Option<String>
}

type Query {
  users(pagination: Option<PaginationInput>): Connection<User>
  posts(pagination: Option<PaginationInput>): Connection<Post>
}
```

### Sorting

```graphql
enum SortDirection {
  ASC
  DESC
}

input SortInput {
  field: String
  direction: SortDirection = ASC
}

type Query {
  users(sort: Option<List<SortInput>>): List<User>
}
```

### Filtering

```graphql
input UserFilterInput {
  name: Option<StringFilter>
  email: Option<StringFilter>
  role: Option<UserRole>
  createdAt: Option<DateTimeFilter>
  AND: Option<List<UserFilterInput>>
  OR: Option<List<UserFilterInput>>
  NOT: Option<UserFilterInput>
}

input StringFilter {
  equals: Option<String>
  contains: Option<String>
  startsWith: Option<String>
  endsWith: Option<String>
}

input DateTimeFilter {
  equals: Option<DateTime>
  gt: Option<DateTime>
  gte: Option<DateTime>
  lt: Option<DateTime>
  lte: Option<DateTime>
}
```

## TypeScript Generation

```graphql
input CreateUserInput {
  name: String
  email: String
  role: UserRole = USER
  tags: Option<List<String>>
}
```

```typescript
// Generated TypeScript
export interface CreateUserInput {
  readonly name: string;
  readonly email: string;
  readonly role?: UserRole;
  readonly tags?: readonly string[] | null;
}
```

## Validation Error Handling

```graphql
type ValidationError {
  field: String
  message: String
  code: String
}

type ValidationErrors {
  errors: List<ValidationError>
}

union CreateUserResult = User | ValidationErrors

type Mutation {
  createUser(input: CreateUserInput): CreateUserResult
}
```

```typescript
const result = await client.mutate(CreateUser, { input });

if (result.createUser.__typename === 'ValidationErrors') {
  result.createUser.errors.forEach(err => {
    setFieldError(err.field, err.message);
  });
} else {
  router.push(`/users/${result.createUser.id}`);
}
```

## Best Practices

### 1. Use Specific Input Types

```graphql
# ✅ Good: Specific input types
input CreateUserInput {
  name: String
  email: String
  password: String
}

input UpdateUserInput {
  name: Option<String>
  email: Option<String>
}

# ❌ Avoid: Generic input for everything
input UserInput {
  id: Option<ID>
  name: Option<String>
  email: Option<String>
  password: Option<String>
}
```

### 2. Validate at Schema Level

```graphql
# ✅ Good: Schema-level validation
input CreatePostInput {
  title: String @minLength(1) @maxLength(200)
  content: String @maxLength(50000)
}

# ❌ Avoid: No validation
input CreatePostInput {
  title: String
  content: String
}
```

### 3. Use Sensible Defaults

```graphql
# ✅ Good: Reasonable defaults
input ListUsersInput {
  first: Int = 20
  offset: Int = 0
  includeInactive: Boolean = false
}
```

### 4. Group Related Fields

```graphql
# ✅ Good: Related fields grouped
input CreateOrderInput {
  customer: CustomerInput
  shipping: ShippingInput
  items: List<OrderItemInput>
}

# ❌ Avoid: Flat structure with many fields
input CreateOrderInput {
  customerName: String
  customerEmail: String
  customerPhone: String
  shippingStreet: String
  shippingCity: String
  # ...30 more fields
}
```

## Next Steps

- [Generics](/schema/generics)
- [Directives](/schema/directives)
- [Types](/schema/types)
