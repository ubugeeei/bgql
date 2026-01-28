<script setup lang="ts">
import { useQuery } from '@bgql/client/vue'
import { BgqlDefer, BgqlStream, BgqlBinaryStream } from '@bgql/client/vue'
import type { DocumentNode } from '@bgql/client/vue'

// Example query with @defer and @stream
const DASHBOARD_QUERY: DocumentNode = {
  kind: 'Document',
  definitions: [
    {
      kind: 'OperationDefinition',
      operation: 'query',
      name: { kind: 'Name', value: 'GetDashboard' },
      selectionSet: {
        kind: 'SelectionSet',
        selections: [],
      },
    },
  ],
}

interface DashboardData {
  user: {
    id: string
    name: string
    avatarUrl: string
    stats?: {
      postsCount: number
      followersCount: number
    }
  }
  feed: Array<{
    id: string
    title: string
    content: string
  }>
}

const { data, loading, error, streamState } = useQuery<DashboardData>(
  DASHBOARD_QUERY,
  {
    fetchPolicy: 'cache-and-network',
  }
)
</script>

<template>
  <div class="app">
    <header>
      <h1>BGQL Streaming Demo</h1>
    </header>

    <main>
      <div v-if="loading && !data">Loading...</div>
      <div v-else-if="error">Error: {{ error.message }}</div>

      <template v-else-if="data">
        <!-- User Profile Section -->
        <section class="user-profile">
          <img :src="data.user.avatarUrl" :alt="data.user.name" />
          <h2>{{ data.user.name }}</h2>

          <!-- Deferred Stats (@defer) -->
          <BgqlDefer label="stats">
            <template #default>
              <div class="stats">
                <span>{{ data.user.stats?.postsCount }} posts</span>
                <span>{{ data.user.stats?.followersCount }} followers</span>
              </div>
            </template>
            <template #fallback>
              <div class="stats-skeleton">Loading stats...</div>
            </template>
          </BgqlDefer>
        </section>

        <!-- Feed Section (@stream) -->
        <section class="feed">
          <h3>Feed</h3>
          <BgqlStream
            label="feed"
            :items="data.feed"
            v-slot="{ item }"
          >
            <article class="post">
              <h4>{{ item.title }}</h4>
              <p>{{ item.content }}</p>
            </article>
          </BgqlStream>
        </section>

        <!-- Stream State Debug Info -->
        <aside class="debug" v-if="streamState.hasNext">
          <p>Streaming in progress...</p>
          <p>Pending defers: {{ streamState.pendingDefers.join(', ') || 'none' }}</p>
        </aside>
      </template>
    </main>
  </div>
</template>

<style scoped>
.app {
  font-family: system-ui, sans-serif;
  max-width: 800px;
  margin: 0 auto;
  padding: 2rem;
}

.user-profile {
  display: flex;
  align-items: center;
  gap: 1rem;
  padding: 1rem;
  border-radius: 8px;
  background: #f5f5f5;
}

.user-profile img {
  width: 64px;
  height: 64px;
  border-radius: 50%;
}

.stats {
  display: flex;
  gap: 1rem;
  margin-top: 0.5rem;
  color: #666;
}

.stats-skeleton {
  color: #999;
  font-style: italic;
}

.feed {
  margin-top: 2rem;
}

.post {
  padding: 1rem;
  margin-bottom: 1rem;
  border: 1px solid #ddd;
  border-radius: 4px;
}

.debug {
  margin-top: 2rem;
  padding: 1rem;
  background: #fff3cd;
  border-radius: 4px;
  font-size: 0.875rem;
}
</style>
