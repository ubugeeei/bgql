/**
 * BgqlDefer Component
 *
 * Suspense boundary component for @defer fragments.
 */

import {
  defineComponent,
  inject,
  computed,
  h,
<<<<<<< HEAD
  type PropType,
  type VNode,
=======
  ref,
  watch,
  onMounted,
  type PropType,
  type VNode,
  type ComputedRef,
>>>>>>> 703747c251d776e50c5464e836b0be66b7f8ebc9
} from 'vue';
import type { BgqlQueryContext, StreamState } from '../types';

/**
 * Injection key for BGQL query context.
 */
export const BGQL_QUERY_CONTEXT_KEY = Symbol('bgql-query-context');

/**
<<<<<<< HEAD
=======
 * Props for the BgqlDefer component.
 */
export interface BgqlDeferProps {
  /**
   * The label of the deferred fragment.
   * Must match the label in @defer(label: "...").
   */
  label: string;

  /**
   * Fragment name for data extraction.
   * If provided, extracts specific fragment data from the response.
   */
  fragment?: string;

  /**
   * Timeout in milliseconds before showing fallback.
   * If data arrives before timeout, fallback is skipped.
   */
  timeout?: number;

  /**
   * Whether to keep showing fallback until all data is loaded.
   */
  waitForAll?: boolean;

  /**
   * Minimum time to show fallback (prevents flash of loading state).
   */
  minLoadingTime?: number;
}

/**
>>>>>>> 703747c251d776e50c5464e836b0be66b7f8ebc9
 * BgqlDefer component for rendering deferred content.
 *
 * This component acts as a suspense boundary for @defer fragments,
 * showing a fallback while the deferred data is loading.
 *
 * @example
 * ```vue
 * <template>
 *   <BgqlProvider :query="query">
 *     <h1>{{ data.user.name }}</h1>
 *
<<<<<<< HEAD
=======
 *     <!-- Basic usage with label -->
>>>>>>> 703747c251d776e50c5464e836b0be66b7f8ebc9
 *     <BgqlDefer label="bio">
 *       <template #default="{ data }">
 *         <p>{{ data.bio }}</p>
 *       </template>
 *       <template #fallback>
 *         <Skeleton />
 *       </template>
 *     </BgqlDefer>
<<<<<<< HEAD
=======
 *
 *     <!-- With fragment name for specific data extraction -->
 *     <BgqlDefer label="posts" fragment="UserPosts">
 *       <template #default="{ data: posts }">
 *         <PostList :posts="posts" />
 *       </template>
 *       <template #fallback>
 *         <PostListSkeleton />
 *       </template>
 *       <template #error="{ error }">
 *         <ErrorMessage :error="error" />
 *       </template>
 *     </BgqlDefer>
 *
 *     <!-- Wait for all deferred data -->
 *     <BgqlDefer label="analytics" :waitForAll="true">
 *       <AnalyticsDashboard />
 *       <template #fallback>
 *         <LoadingDashboard />
 *       </template>
 *     </BgqlDefer>
>>>>>>> 703747c251d776e50c5464e836b0be66b7f8ebc9
 *   </BgqlProvider>
 * </template>
 * ```
 */
export const BgqlDefer = defineComponent({
  name: 'BgqlDefer',

  props: {
    /**
     * The label of the deferred fragment.
     * Must match the label in @defer(label: "...").
     */
    label: {
      type: String,
      required: true,
    },

    /**
<<<<<<< HEAD
=======
     * Fragment name for data extraction.
     */
    fragment: {
      type: String,
      default: undefined,
    },

    /**
>>>>>>> 703747c251d776e50c5464e836b0be66b7f8ebc9
     * Timeout in milliseconds before showing fallback.
     * If data arrives before timeout, fallback is skipped.
     */
    timeout: {
      type: Number,
      default: 0,
    },

    /**
     * Whether to keep showing fallback until all data is loaded.
     */
    waitForAll: {
      type: Boolean,
      default: false,
    },
<<<<<<< HEAD
  },

  setup(props, { slots }) {
    const context = inject<BgqlQueryContext>(BGQL_QUERY_CONTEXT_KEY);

    if (!context) {
=======

    /**
     * Minimum time to show fallback (prevents flash of loading state).
     */
    minLoadingTime: {
      type: Number,
      default: 0,
    },
  },

  setup(props, { slots }) {
    // Inject context - can be either direct context or computed
    const injectedContext = inject<BgqlQueryContext | ComputedRef<BgqlQueryContext> | null>(
      BGQL_QUERY_CONTEXT_KEY,
      null
    );

    // Normalize context access
    const context = computed<BgqlQueryContext | null>(() => {
      if (!injectedContext) return null;
      // Handle both direct object and computed ref
      if ('value' in injectedContext && typeof injectedContext.value === 'object') {
        return injectedContext.value as BgqlQueryContext;
      }
      return injectedContext as BgqlQueryContext;
    });

    if (!context.value) {
>>>>>>> 703747c251d776e50c5464e836b0be66b7f8ebc9
      console.warn(
        'BgqlDefer must be used within a BgqlProvider or component that provides bgql-query-context'
      );
    }

<<<<<<< HEAD
    const isLoaded = computed(() => {
      if (!context) {
        return true;
      }

      const streamState = context.streamState as StreamState;
=======
    // Track minimum loading time
    const loadedAt = ref<number | null>(null);
    const canShow = ref(false);

    // Check if the fragment is loaded
    const isLoaded = computed(() => {
      if (!context.value) {
        return true;
      }

      const streamState = context.value.streamState as StreamState;
>>>>>>> 703747c251d776e50c5464e836b0be66b7f8ebc9
      const pending = streamState.pendingDefers;

      if (props.waitForAll) {
        return pending.length === 0 && !streamState.hasNext;
      }

      return !pending.includes(props.label);
    });

<<<<<<< HEAD
    const hasError = computed(() => {
      return context?.error !== null;
=======
    // Handle minimum loading time
    watch(isLoaded, (loaded: boolean, wasLoaded: boolean | undefined) => {
      if (loaded && !wasLoaded) {
        loadedAt.value = Date.now();

        if (props.minLoadingTime > 0) {
          // Calculate remaining time to show fallback
          const streamProgress = (context.value?.streamState as StreamState | undefined)?.progress;
          const elapsed = loadedAt.value - (streamProgress ?? 0);
          const remaining = Math.max(0, props.minLoadingTime - elapsed);

          if (remaining > 0) {
            setTimeout(() => {
              canShow.value = true;
            }, remaining);
          } else {
            canShow.value = true;
          }
        } else {
          canShow.value = true;
        }
      }
    }, { immediate: true });

    // If no minimum loading time, allow immediate show
    if (props.minLoadingTime === 0) {
      canShow.value = true;
    }

    const shouldShowContent = computed(() => {
      return isLoaded.value && canShow.value;
    });

    const hasError = computed(() => {
      return context.value?.error !== null;
    });

    /**
     * Extract fragment-specific data if fragment prop is provided.
     */
    const fragmentData = computed(() => {
      if (!context.value?.data || !props.fragment) {
        return context.value?.data;
      }

      // Try to extract fragment data by name
      const data = context.value.data as Record<string, unknown>;

      // Check for __fragments field (common pattern)
      if (data.__fragments && typeof data.__fragments === 'object') {
        const fragments = data.__fragments as Record<string, unknown>;
        if (props.fragment in fragments) {
          return fragments[props.fragment];
        }
      }

      // Try direct property access
      if (props.fragment in data) {
        return data[props.fragment];
      }

      // Return full data as fallback
      return data;
>>>>>>> 703747c251d776e50c5464e836b0be66b7f8ebc9
    });

    return () => {
      // Error state
      if (hasError.value && slots.error) {
<<<<<<< HEAD
        return slots.error({ error: context?.error });
      }

      // Loading state
      if (!isLoaded.value) {
        if (slots.fallback) {
          return slots.fallback();
        }
        // Default fallback
        return h('div', { class: 'bgql-defer-loading' }, 'Loading...');
      }

      // Loaded state
      if (slots.default) {
        return slots.default({ data: context?.data });
=======
        return slots.error({ error: context.value?.error });
      }

      // Loading state - show fallback
      if (!shouldShowContent.value) {
        if (slots.fallback) {
          return slots.fallback();
        }
        // Default fallback with accessible loading indicator
        return h('div', {
          class: 'bgql-defer-loading',
          role: 'status',
          'aria-busy': 'true',
          'aria-label': 'Loading...',
        }, [
          h('span', { class: 'bgql-defer-loading-text' }, 'Loading...'),
        ]);
      }

      // Loaded state - show content
      if (slots.default) {
        return slots.default({
          data: fragmentData.value,
          loading: false,
        });
>>>>>>> 703747c251d776e50c5464e836b0be66b7f8ebc9
      }

      return null;
    };
  },
});

/**
 * Type-safe version of BgqlDefer for TypeScript.
<<<<<<< HEAD
=======
 *
 * @example
 * ```typescript
 * interface UserBio {
 *   bio: string
 *   socialLinks: string[]
 * }
 *
 * const UserBioDefer = createTypedDefer<UserBio>()
 *
 * // In template:
 * // <UserBioDefer label="bio" v-slot="{ data }">
 * //   <p>{{ data.bio }}</p>
 * // </UserBioDefer>
 * ```
>>>>>>> 703747c251d776e50c5464e836b0be66b7f8ebc9
 */
export function createTypedDefer<TData>() {
  return BgqlDefer as typeof BgqlDefer & {
    new (): {
      $slots: {
<<<<<<< HEAD
        default: (props: { data: TData }) => VNode[];
=======
        default: (props: { data: TData; loading: boolean }) => VNode[];
>>>>>>> 703747c251d776e50c5464e836b0be66b7f8ebc9
        fallback: () => VNode[];
        error: (props: { error: Error }) => VNode[];
      };
    };
  };
}

export default BgqlDefer;
