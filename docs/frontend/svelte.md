# Svelte Integration

Better GraphQL provides Svelte stores and utilities for reactive GraphQL operations.

## Setup

### Installation

```bash
bun add @bgql/client
```

### Client Configuration

```typescript
// src/lib/graphql.ts
import { createClient } from '@bgql/client';

export const client = createClient('http://localhost:4000/graphql', {
  headers: () => {
    const token = localStorage.getItem('token');
    return token ? { Authorization: `Bearer ${token}` } : {};
  },
});
```

### Context Setup

```svelte
<!-- src/routes/+layout.svelte -->
<script lang="ts">
  import { setContext } from 'svelte';
  import { client } from '$lib/graphql';

  setContext('graphql', client);
</script>

<slot />
```

## Query Store

### Basic Query

```svelte
<script lang="ts">
  import { query } from '@bgql/client/svelte';
  import { GetUserDocument } from '$lib/generated/graphql';

  export let userId: string;

  const user = query(GetUserDocument, { variables: { id: userId } });
</script>

{#if $user.loading}
  <p>Loading...</p>
{:else if $user.error}
  <p>Error: {$user.error.message}</p>
{:else}
  <h1>{$user.data.user.name}</h1>
  <p>{$user.data.user.email}</p>
{/if}
```

### Reactive Variables

```svelte
<script lang="ts">
  import { query } from '@bgql/client/svelte';
  import { GetUserDocument } from '$lib/generated/graphql';

  export let userId: string;

  // Reactively refetches when userId changes
  $: user = query(GetUserDocument, { variables: { id: userId } });
</script>

{#if $user.data}
  <UserProfile user={$user.data.user} />
{/if}
```

### Query Options

```svelte
<script lang="ts">
  const user = query(GetUserDocument, {
    variables: { id: '1' },

    // Fetch policy
    fetchPolicy: 'cache-first',

    // Poll interval (ms)
    pollInterval: 30000,

    // Skip query
    skip: !isLoggedIn,
  });

  // Manual refetch
  function refresh() {
    user.refetch();
  }
</script>
```

### Lazy Query

```svelte
<script lang="ts">
  import { lazyQuery } from '@bgql/client/svelte';

  const search = lazyQuery(SearchUsersDocument);

  async function handleSearch(query: string) {
    await search.execute({ variables: { query } });
  }
</script>

<input on:input={(e) => handleSearch(e.currentTarget.value)} />

{#if $search.data}
  <SearchResults results={$search.data.search} />
{/if}
```

## Mutation Store

### Basic Mutation

```svelte
<script lang="ts">
  import { mutation } from '@bgql/client/svelte';
  import { matchUnion } from '@bgql/client';
  import { CreateUserDocument } from '$lib/generated/graphql';

  const createUser = mutation(CreateUserDocument);

  let name = '';
  let email = '';
  let errors: Record<string, string> = {};

  async function handleSubmit() {
    errors = {};

    const result = await createUser.mutate({
      input: { name, email },
    });

    if (result.ok) {
      matchUnion(result.value.createUser, {
        User: (user) => {
          goto(`/users/${user.id}`);
        },
        ValidationError: (error) => {
          errors[error.field] = error.message;
        },
      });
    }
  }
</script>

<form on:submit|preventDefault={handleSubmit}>
  <input bind:value={name} placeholder="Name" />
  {#if errors.name}<span class="error">{errors.name}</span>{/if}

  <input bind:value={email} type="email" placeholder="Email" />
  {#if errors.email}<span class="error">{errors.email}</span>{/if}

  <button type="submit" disabled={$createUser.loading}>
    {$createUser.loading ? 'Creating...' : 'Create User'}
  </button>
</form>
```

### Optimistic Updates

```svelte
<script lang="ts">
  const likePost = mutation(LikePostDocument, {
    optimisticResponse: (variables) => ({
      likePost: {
        __typename: 'Post',
        id: variables.postId,
        liked: true,
        likeCount: getCurrentLikeCount(variables.postId) + 1,
      },
    }),
  });
</script>
```

## Subscription Store

### Basic Subscription

```svelte
<script lang="ts">
  import { subscription } from '@bgql/client/svelte';
  import { MessageCreatedDocument } from '$lib/generated/graphql';

  export let channelId: string;

  let messages: Message[] = [];

  const sub = subscription(MessageCreatedDocument, {
    variables: { channelId },
    onData: (data) => {
      messages = [...messages, data.messageCreated];
    },
  });
</script>

{#if $sub.loading}
  <p>Connecting...</p>
{:else if $sub.error}
  <p>Error: {$sub.error.message}</p>
{:else}
  {#each messages as message (message.id)}
    <Message {message} />
  {/each}
{/if}
```

## Type-Safe Union Handling

### matchUnion

```svelte
<script lang="ts">
  import { query } from '@bgql/client/svelte';
  import { matchUnion, isTypename } from '@bgql/client';

  const user = query(GetUserDocument, { variables: { id: '1' } });
</script>

{#if $user.data}
  {#if isTypename('User')($user.data.user)}
    <UserProfile user={$user.data.user} />
  {:else if isTypename('NotFoundError')($user.data.user)}
    <NotFound message={$user.data.user.message} />
  {:else if isTypename('AuthError')($user.data.user)}
    <AuthRequired />
  {/if}
{/if}
```

## Error Handling

### Error Store

```svelte
<script lang="ts">
  const user = query(GetUserDocument, {
    variables: { id: '1' },
    onError: (error) => {
      if (error.extensions?.code === 'UNAUTHENTICATED') {
        goto('/login');
      }
    },
  });
</script>

{#if $user.error}
  <div class="error">
    <p>{$user.error.message}</p>
    <button on:click={() => user.refetch()}>Retry</button>
  </div>
{/if}
```

## Pagination

### Cursor-Based

```svelte
<script lang="ts">
  import { query } from '@bgql/client/svelte';

  let cursor: string | null = null;
  let allPosts: Post[] = [];

  const posts = query(GetPostsDocument, {
    variables: { first: 10, after: cursor },
  });

  $: if ($posts.data) {
    allPosts = [
      ...allPosts,
      ...$posts.data.posts.edges.map(e => e.node),
    ];
  }

  function loadMore() {
    if ($posts.data?.posts.pageInfo.hasNextPage) {
      cursor = $posts.data.posts.pageInfo.endCursor;
    }
  }
</script>

{#each allPosts as post (post.id)}
  <PostCard {post} />
{/each}

{#if $posts.data?.posts.pageInfo.hasNextPage}
  <button on:click={loadMore} disabled={$posts.loading}>
    {$posts.loading ? 'Loading...' : 'Load More'}
  </button>
{/if}
```

## SvelteKit Integration

### Server-Side Fetching

```typescript
// src/routes/users/[id]/+page.server.ts
import type { PageServerLoad } from './$types';
import { client } from '$lib/graphql';
import { GetUserDocument } from '$lib/generated/graphql';

export const load: PageServerLoad = async ({ params }) => {
  const result = await client.query(GetUserDocument, { id: params.id });

  if (!result.ok) {
    throw error(500, result.error.message);
  }

  return {
    user: result.value.user,
  };
};
```

```svelte
<!-- src/routes/users/[id]/+page.svelte -->
<script lang="ts">
  import type { PageData } from './$types';

  export let data: PageData;
</script>

<UserProfile user={data.user} />
```

## Best Practices

### 1. Use Reactive Statements

```svelte
<script lang="ts">
  export let userId: string;

  // Automatically refetches when userId changes
  $: user = query(GetUserDocument, { variables: { id: userId } });
</script>
```

### 2. Handle All States

```svelte
{#if $user.loading}
  <Skeleton />
{:else if $user.error}
  <Error error={$user.error} on:retry={() => user.refetch()} />
{:else if $user.data}
  <UserProfile user={$user.data.user} />
{/if}
```

### 3. Clean Up Subscriptions

```svelte
<script lang="ts">
  import { onDestroy } from 'svelte';

  const sub = subscription(MessageCreatedDocument, {
    variables: { channelId },
  });

  onDestroy(() => {
    sub.unsubscribe();
  });
</script>
```

## Next Steps

- [Queries](/frontend/queries)
- [Mutations](/frontend/mutations)
- [Type Safety](/frontend/type-safety)
