# Mutations

Learn how to modify data with Better GraphQL's type-safe mutation system.

## Basic Mutations

### Simple Mutation

```typescript
import { createClient } from '@bgql/client';

const client = createClient('http://localhost:4000/graphql');

const result = await client.mutate(`
  mutation CreateUser($input: CreateUserInput!) {
    createUser(input: $input) {
      id
      name
    }
  }
`, {
  input: {
    name: 'John Doe',
    email: 'john@example.com',
  },
});

if (result.ok) {
  console.log('Created user:', result.value.createUser.id);
}
```

## Typed Mutations

### Using TypedDocumentNode

```typescript
import { gql } from '@bgql/client';

const CreateUserDocument = gql<
  { createUser: User | ValidationError },
  { input: CreateUserInput }
>`
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
`;

const result = await client.mutateTyped(CreateUserDocument, {
  input: {
    name: 'John Doe',
    email: 'john@example.com',
  },
});
```

### Generated Types (Recommended)

```typescript
import {
  CreateUserDocument,
  UpdateUserDocument,
  DeleteUserDocument,
} from './generated/graphql';

const result = await client.mutateTyped(CreateUserDocument, {
  input: { name: 'John', email: 'john@example.com' },
});
```

## Handling Results

### Union Results

Most mutations return union types for type-safe error handling:

```graphql
# Schema
union CreateUserResult = User | ValidationError | EmailExistsError

type Mutation {
  createUser(input: CreateUserInput!): CreateUserResult!
}
```

```typescript
import { matchUnion, isTypename } from '@bgql/client';

const result = await client.mutateTyped(CreateUserDocument, { input });

if (result.ok) {
  matchUnion(result.value.createUser, {
    User: (user) => {
      toast.success(`Created user ${user.name}`);
      router.push(`/users/${user.id}`);
    },
    ValidationError: (error) => {
      setFieldError(error.field, error.message);
    },
    EmailExistsError: (error) => {
      setFieldError('email', 'This email is already registered');
    },
  });
}
```

### Multiple Validation Errors

```graphql
# Schema
type ValidationErrors {
  errors: [ValidationError!]!
}

union CreateUserResult = User | ValidationErrors
```

```typescript
if (result.ok) {
  matchUnion(result.value.createUser, {
    User: (user) => {
      // Success
    },
    ValidationErrors: ({ errors }) => {
      errors.forEach(error => {
        setFieldError(error.field, error.message);
      });
    },
  });
}
```

## Form Integration

### React Hook Form

```typescript
import { useForm } from 'react-hook-form';

function CreateUserForm() {
  const { register, handleSubmit, setError } = useForm<CreateUserInput>();

  const onSubmit = async (data: CreateUserInput) => {
    const result = await client.mutateTyped(CreateUserDocument, {
      input: data,
    });

    if (result.ok) {
      matchUnion(result.value.createUser, {
        User: (user) => {
          toast.success('User created!');
          router.push(`/users/${user.id}`);
        },
        ValidationError: (error) => {
          setError(error.field as keyof CreateUserInput, {
            message: error.message,
          });
        },
      });
    } else {
      toast.error('Network error. Please try again.');
    }
  };

  return (
    <form onSubmit={handleSubmit(onSubmit)}>
      <input {...register('name')} />
      <input {...register('email')} type="email" />
      <button type="submit">Create User</button>
    </form>
  );
}
```

### Vue Form

```vue
<script setup lang="ts">
import { ref } from 'vue';
import { useMutation } from '@bgql/client/vue';
import { CreateUserDocument } from './generated/graphql';

const name = ref('');
const email = ref('');
const errors = ref<Record<string, string>>({});

const { mutate, loading } = useMutation(CreateUserDocument, {
  onCompleted: (data) => {
    matchUnion(data.createUser, {
      User: (user) => {
        router.push(`/users/${user.id}`);
      },
      ValidationError: (error) => {
        errors.value[error.field] = error.message;
      },
    });
  },
});

async function handleSubmit() {
  errors.value = {};
  await mutate({ input: { name: name.value, email: email.value } });
}
</script>

<template>
  <form @submit.prevent="handleSubmit">
    <input v-model="name" />
    <span v-if="errors.name">{{ errors.name }}</span>

    <input v-model="email" type="email" />
    <span v-if="errors.email">{{ errors.email }}</span>

    <button type="submit" :disabled="loading">
      {{ loading ? 'Creating...' : 'Create User' }}
    </button>
  </form>
</template>
```

## Optimistic Updates

### Basic Optimistic Update

```typescript
import { useMutation } from '@bgql/client/vue';

const { mutate } = useMutation(UpdateUserDocument, {
  optimisticResponse: (variables) => ({
    updateUser: {
      __typename: 'User',
      id: variables.id,
      ...variables.input,
    },
  }),
});
```

### With Cache Update

```typescript
const { mutate } = useMutation(UpdateUserDocument, {
  optimisticResponse: (variables) => ({
    updateUser: {
      __typename: 'User',
      id: variables.id,
      ...variables.input,
    },
  }),
  update: (cache, { data }) => {
    if (data?.updateUser.__typename === 'User') {
      cache.update('User', data.updateUser.id, data.updateUser);
    }
  },
});
```

## Delete Operations

### Basic Delete

```typescript
const DeleteUserDocument = gql<
  { deleteUser: boolean | NotFoundError },
  { id: string }
>`
  mutation DeleteUser($id: ID!) {
    deleteUser(id: $id) {
      ... on Boolean {
        value
      }
      ... on NotFoundError {
        message
      }
    }
  }
`;

const result = await client.mutateTyped(DeleteUserDocument, { id: '1' });
```

### With Confirmation

```typescript
async function handleDelete(userId: string) {
  const confirmed = await confirm('Are you sure you want to delete this user?');
  if (!confirmed) return;

  const result = await client.mutateTyped(DeleteUserDocument, { id: userId });

  if (result.ok) {
    if (typeof result.value.deleteUser === 'boolean') {
      toast.success('User deleted');
      router.push('/users');
    } else {
      toast.error(result.value.deleteUser.message);
    }
  }
}
```

## Batch Mutations

### Sequential Mutations

```typescript
async function createUsersSequentially(inputs: CreateUserInput[]) {
  const results = [];

  for (const input of inputs) {
    const result = await client.mutateTyped(CreateUserDocument, { input });
    results.push(result);

    if (!result.ok) {
      // Stop on first error
      break;
    }
  }

  return results;
}
```

### Parallel Mutations

```typescript
async function createUsersInParallel(inputs: CreateUserInput[]) {
  const results = await Promise.all(
    inputs.map(input =>
      client.mutateTyped(CreateUserDocument, { input })
    )
  );

  const succeeded = results.filter(r => r.ok);
  const failed = results.filter(r => !r.ok);

  return { succeeded, failed };
}
```

## File Uploads

### Single File

```typescript
const UploadAvatarDocument = gql<
  { uploadAvatar: { url: string } },
  { userId: string; file: File }
>`
  mutation UploadAvatar($userId: ID!, $file: Upload!) {
    uploadAvatar(userId: $userId, file: $file) {
      url
    }
  }
`;

async function handleFileUpload(event: Event) {
  const file = (event.target as HTMLInputElement).files?.[0];
  if (!file) return;

  const result = await client.mutateTyped(UploadAvatarDocument, {
    userId: currentUser.id,
    file,
  });

  if (result.ok) {
    setAvatarUrl(result.value.uploadAvatar.url);
  }
}
```

### Multiple Files

```typescript
async function uploadFiles(files: File[]) {
  const results = await Promise.all(
    files.map(file =>
      client.mutateTyped(UploadFileDocument, { file })
    )
  );

  return results
    .filter(r => r.ok)
    .map(r => r.value.uploadFile.url);
}
```

## Error Handling

### Network Errors

```typescript
const result = await client.mutateTyped(CreateUserDocument, { input });

if (!result.ok) {
  switch (result.error.type) {
    case 'network':
      toast.error('Network error. Please check your connection.');
      break;
    case 'timeout':
      toast.error('Request timed out. Please try again.');
      break;
    default:
      toast.error('An unexpected error occurred.');
  }
  return;
}
```

### GraphQL Errors

```typescript
if (!result.ok && result.error.type === 'graphql') {
  if (result.error.extensions?.code === 'UNAUTHENTICATED') {
    router.push('/login');
  } else {
    toast.error(result.error.message);
  }
}
```

## Loading States

### Basic Loading State

```typescript
const [loading, setLoading] = useState(false);

async function handleSubmit() {
  setLoading(true);
  try {
    const result = await client.mutateTyped(CreateUserDocument, { input });
    // Handle result
  } finally {
    setLoading(false);
  }
}
```

### With Vue Composable

```typescript
const { mutate, loading, error } = useMutation(CreateUserDocument);

// loading is reactive
// error is reactive
```

## Best Practices

### 1. Use Union Types for Results

```graphql
# ✅ Good: Typed error handling
union CreateUserResult = User | ValidationError | EmailExistsError

type Mutation {
  createUser(input: CreateUserInput!): CreateUserResult!
}
```

### 2. Handle All Cases

```typescript
// ✅ Good: Handle all cases
matchUnion(result.value.createUser, {
  User: handleSuccess,
  ValidationError: handleValidation,
  EmailExistsError: handleEmailExists,
});

// ❌ Avoid: Ignoring error cases
if (result.value.createUser.__typename === 'User') {
  // What about errors?
}
```

### 3. Show Loading States

```typescript
// ✅ Good: Disable button during mutation
<button disabled={loading}>
  {loading ? 'Creating...' : 'Create User'}
</button>
```

### 4. Provide Feedback

```typescript
// ✅ Good: Show success/error feedback
if (result.ok) {
  toast.success('User created successfully!');
} else {
  toast.error('Failed to create user. Please try again.');
}
```

## Next Steps

- [Queries](/frontend/queries)
- [Type Safety](/frontend/type-safety)
- [Vue.js Integration](/frontend/vue)
