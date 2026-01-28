/**
 * Vue Components for BGQL
 *
 * Re-exports all BGQL Vue components.
 *
 * @example
 * ```vue
 * <script setup>
 * import { Bgql } from '@bgql/client/vue'
 * </script>
 *
 * <template>
 *   <Bgql.Provider :query="query">
 *     <Bgql.Defer label="stats">
 *       <Stats />
 *     </Bgql.Defer>
 *     <Bgql.Stream label="feed" :items="data.feed" v-slot="{ item }">
 *       <PostCard :post="item" />
 *     </Bgql.Stream>
 *   </Bgql.Provider>
 * </template>
 * ```
 */

import { BgqlDefer as DeferComponent, createTypedDefer, BGQL_QUERY_CONTEXT_KEY } from './BgqlDefer';
import { BgqlStream as StreamComponent, createTypedStream } from './BgqlStream';
import { BgqlBinaryStream as BinaryStreamComponent } from './BgqlBinaryStream';
import { BgqlProvider as ProviderComponent } from './BgqlProvider';

// Named exports for individual component imports
export { createTypedDefer, BGQL_QUERY_CONTEXT_KEY } from './BgqlDefer';
export { createTypedStream } from './BgqlStream';

// Legacy named exports (for backwards compatibility)
export const BgqlDefer = DeferComponent;
export const BgqlStream = StreamComponent;
export const BgqlBinaryStream = BinaryStreamComponent;
export const BgqlProvider = ProviderComponent;

/**
 * Barrel export for BGQL Vue components.
 *
 * Provides a namespace-style API for cleaner imports:
 * - `Bgql.Defer` - Deferred content boundary
 * - `Bgql.Stream` - Streaming list component
 * - `Bgql.BinaryStream` - Binary stream (video/audio) component
 * - `Bgql.Provider` - Query context provider
 *
 * @example
 * ```vue
 * <script setup>
 * import { Bgql } from '@bgql/client/vue'
 * </script>
 *
 * <template>
 *   <Bgql.Defer label="bio">
 *     <UserBio />
 *     <template #fallback><Skeleton /></template>
 *   </Bgql.Defer>
 * </template>
 * ```
 */
export const Bgql = {
  /** Deferred content boundary component */
  Defer: DeferComponent,
  /** Streaming list component */
  Stream: StreamComponent,
  /** Binary stream (video/audio) component */
  BinaryStream: BinaryStreamComponent,
  /** Query context provider component */
  Provider: ProviderComponent,
} as const;
