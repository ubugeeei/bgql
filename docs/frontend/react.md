# React Integration

Better GraphQL provides React hooks for seamless data fetching with full TypeScript support.

## Setup

### Installation

```bash
bun add @bgql/client
```

### Client Configuration

```typescript
// graphql.ts
import { createClient } from '@bgql/client';

export const client = createClient('http://localhost:4000/graphql', {
  headers: () => {
    const token = localStorage.getItem('token');
    return token ? { Authorization: `Bearer ${token}` } : {};
  },
});
```

### Provider Setup

```tsx
// App.tsx
import { GraphQLProvider } from '@bgql/client/react';
import { client } from './graphql';

function App() {
  return (
    <GraphQLProvider client={client}>
      <Router />
    </GraphQLProvider>
  );
}
```

## useQuery

### Basic Query

```tsx
import { useQuery } from '@bgql/client/react';
import { GetUserDocument } from './generated/graphql';

function UserProfile({ id }: { id: string }) {
  const { data, loading, error } = useQuery(GetUserDocument, {
    variables: { id },
  });

  if (loading) return <Spinner />;
  if (error) return <Error message={error.message} />;

  return (
    <div>
      <h1>{data.user.name}</h1>
      <p>{data.user.email}</p>
    </div>
  );
}
```

### Query with Options

```tsx
const { data, loading, error, refetch } = useQuery(GetUserDocument, {
  variables: { id: '1' },

  // Fetch policy
  fetchPolicy: 'cache-and-network',

  // Poll interval (ms)
  pollInterval: 30000,

  // Skip query conditionally
  skip: !isLoggedIn,

  // Callbacks
  onCompleted: (data) => {
    console.log('Query completed:', data);
  },
  onError: (error) => {
    console.error('Query failed:', error);
  },
});
```

### Lazy Query

```tsx
function SearchUsers() {
  const [search, { data, loading }] = useLazyQuery(SearchUsersDocument);

  const handleSearch = (query: string) => {
    search({ variables: { query } });
  };

  return (
    <div>
      <input onChange={(e) => handleSearch(e.target.value)} />
      {loading && <Spinner />}
      {data && <SearchResults results={data.search} />}
    </div>
  );
}
```

### Suspense Mode

```tsx
import { Suspense } from 'react';
import { useQuery } from '@bgql/client/react';

function UserProfile({ id }: { id: string }) {
  const { data } = useQuery(GetUserDocument, {
    variables: { id },
    suspense: true,
  });

  // No loading check needed - Suspense handles it
  return <h1>{data.user.name}</h1>;
}

function App() {
  return (
    <Suspense fallback={<Spinner />}>
      <UserProfile id="1" />
    </Suspense>
  );
}
```

## useMutation

### Basic Mutation

```tsx
import { useMutation } from '@bgql/client/react';
import { CreateUserDocument } from './generated/graphql';

function CreateUserForm() {
  const [createUser, { loading, error }] = useMutation(CreateUserDocument);

  const handleSubmit = async (e: FormEvent) => {
    e.preventDefault();
    const formData = new FormData(e.target as HTMLFormElement);

    const result = await createUser({
      variables: {
        input: {
          name: formData.get('name') as string,
          email: formData.get('email') as string,
        },
      },
    });

    if (result.ok) {
      matchUnion(result.value.createUser, {
        User: (user) => {
          toast.success('User created!');
          navigate(`/users/${user.id}`);
        },
        ValidationError: (error) => {
          setError(error.field, { message: error.message });
        },
      });
    }
  };

  return (
    <form onSubmit={handleSubmit}>
      <input name="name" placeholder="Name" />
      <input name="email" type="email" placeholder="Email" />
      <button type="submit" disabled={loading}>
        {loading ? 'Creating...' : 'Create User'}
      </button>
    </form>
  );
}
```

### Optimistic Updates

```tsx
const [likePost] = useMutation(LikePostDocument, {
  optimisticResponse: (variables) => ({
    likePost: {
      __typename: 'Post',
      id: variables.postId,
      liked: true,
      likeCount: getCurrentLikeCount(variables.postId) + 1,
    },
  }),
});
```

### Cache Updates

```tsx
const [createPost] = useMutation(CreatePostDocument, {
  update: (cache, { data }) => {
    if (data?.createPost.__typename === 'Post') {
      const existing = cache.readQuery({ query: GetPostsDocument });
      cache.writeQuery({
        query: GetPostsDocument,
        data: {
          posts: {
            ...existing.posts,
            edges: [
              { node: data.createPost, cursor: data.createPost.id },
              ...existing.posts.edges,
            ],
          },
        },
      });
    }
  },
  refetchQueries: [GetPostsDocument],
});
```

## useSubscription

### Basic Subscription

```tsx
import { useSubscription } from '@bgql/client/react';

function ChatRoom({ channelId }: { channelId: string }) {
  const [messages, setMessages] = useState<Message[]>([]);

  const { loading, error } = useSubscription(MessageCreatedDocument, {
    variables: { channelId },
    onData: (data) => {
      setMessages((prev) => [...prev, data.messageCreated]);
    },
  });

  if (loading) return <div>Connecting...</div>;
  if (error) return <div>Error: {error.message}</div>;

  return <MessageList messages={messages} />;
}
```

## Type-Safe Union Handling

### Using matchUnion

```tsx
import { matchUnion, isTypename } from '@bgql/client';

function UserResult({ result }: { result: UserResultType }) {
  return matchUnion(result, {
    User: (user) => <UserProfile user={user} />,
    NotFoundError: (error) => <NotFound message={error.message} />,
    AuthError: (error) => <AuthRequired message={error.message} />,
  });
}
```

### Type Guards

```tsx
function UserProfile({ id }: { id: string }) {
  const { data } = useQuery(GetUserDocument, { variables: { id } });

  if (!data) return null;

  if (isTypename('User')(data.user)) {
    return <Profile user={data.user} />;
  }

  if (isTypename('NotFoundError')(data.user)) {
    return <NotFound />;
  }

  return <AuthRequired />;
}
```

## Fragments

### Using Fragments

```tsx
import { useFragment } from '@bgql/client/react';
import { UserFieldsFragmentDoc, FragmentType } from './generated/graphql';

interface Props {
  user: FragmentType<typeof UserFieldsFragmentDoc>;
}

function UserCard({ user: userProp }: Props) {
  const user = useFragment(UserFieldsFragmentDoc, userProp);

  return (
    <div className="user-card">
      <img src={user.avatarUrl} alt={user.name} />
      <h3>{user.name}</h3>
      <p>{user.email}</p>
    </div>
  );
}
```

## Error Handling

### Error Boundary

```tsx
import { QueryErrorBoundary } from '@bgql/client/react';

function App() {
  return (
    <QueryErrorBoundary
      fallback={({ error, resetError }) => (
        <div className="error">
          <p>{error.message}</p>
          <button onClick={resetError}>Retry</button>
        </div>
      )}
    >
      <UserProfile id="1" />
    </QueryErrorBoundary>
  );
}
```

### Per-Query Error Handling

```tsx
const { data, error, refetch } = useQuery(GetUserDocument, {
  variables: { id },
  onError: (error) => {
    if (error.extensions?.code === 'UNAUTHENTICATED') {
      navigate('/login');
    }
  },
});

if (error) {
  return (
    <div className="error">
      <p>{error.message}</p>
      <button onClick={() => refetch()}>Retry</button>
    </div>
  );
}
```

## Pagination

### Infinite Query

```tsx
import { useInfiniteQuery } from '@bgql/client/react';

function PostList() {
  const {
    data,
    loading,
    hasNextPage,
    fetchNextPage,
    isFetchingNextPage,
  } = useInfiniteQuery(GetPostsDocument, {
    variables: { first: 10 },
    getNextPageParam: (lastPage) => {
      if (lastPage.posts.pageInfo.hasNextPage) {
        return { after: lastPage.posts.pageInfo.endCursor };
      }
      return undefined;
    },
  });

  const posts = data?.pages.flatMap(page =>
    page.posts.edges.map(e => e.node)
  ) ?? [];

  return (
    <div>
      {posts.map(post => (
        <PostCard key={post.id} post={post} />
      ))}

      {hasNextPage && (
        <button onClick={() => fetchNextPage()} disabled={isFetchingNextPage}>
          {isFetchingNextPage ? 'Loading...' : 'Load More'}
        </button>
      )}
    </div>
  );
}
```

## SSR Support

### Next.js Integration

```tsx
// app/users/[id]/page.tsx
import { createClient } from '@bgql/client';
import { GetUserDocument } from '@/generated/graphql';

const client = createClient(process.env.GRAPHQL_URL!);

export default async function UserPage({ params }: { params: { id: string } }) {
  const result = await client.query(GetUserDocument, { id: params.id });

  if (!result.ok) {
    throw new Error(result.error.message);
  }

  return <UserProfile user={result.value.user} />;
}
```

## Best Practices

### 1. Colocate Queries

```
components/
├── UserProfile/
│   ├── UserProfile.tsx
│   ├── UserProfile.graphql
│   └── index.ts
```

### 2. Use Fragments

```graphql
fragment UserFields on User {
  id
  name
  email
  avatarUrl
}
```

### 3. Handle All States

```tsx
function UserProfile({ id }: { id: string }) {
  const { data, loading, error } = useQuery(GetUserDocument, {
    variables: { id },
  });

  if (loading) return <Skeleton />;
  if (error) return <Error error={error} />;
  if (!data) return null;

  return <Profile user={data.user} />;
}
```

## Next Steps

- [Queries](/frontend/queries)
- [Mutations](/frontend/mutations)
- [Type Safety](/frontend/type-safety)
