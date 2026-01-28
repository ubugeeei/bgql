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
  type PropType,
  type VNode,
} from 'vue';
import type { BgqlQueryContext, StreamState } from '../types';

/**
 * Injection key for BGQL query context.
 */
export const BGQL_QUERY_CONTEXT_KEY = Symbol('bgql-query-context');

/**
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
 *     <BgqlDefer label="bio">
 *       <template #default="{ data }">
 *         <p>{{ data.bio }}</p>
 *       </template>
 *       <template #fallback>
 *         <Skeleton />
 *       </template>
 *     </BgqlDefer>
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
  },

  setup(props, { slots }) {
    const context = inject<BgqlQueryContext>(BGQL_QUERY_CONTEXT_KEY);

    if (!context) {
      console.warn(
        'BgqlDefer must be used within a BgqlProvider or component that provides bgql-query-context'
      );
    }

    const isLoaded = computed(() => {
      if (!context) {
        return true;
      }

      const streamState = context.streamState as StreamState;
      const pending = streamState.pendingDefers;

      if (props.waitForAll) {
        return pending.length === 0 && !streamState.hasNext;
      }

      return !pending.includes(props.label);
    });

    const hasError = computed(() => {
      return context?.error !== null;
    });

    return () => {
      // Error state
      if (hasError.value && slots.error) {
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
      }

      return null;
    };
  },
});

/**
 * Type-safe version of BgqlDefer for TypeScript.
 */
export function createTypedDefer<TData>() {
  return BgqlDefer as typeof BgqlDefer & {
    new (): {
      $slots: {
        default: (props: { data: TData }) => VNode[];
        fallback: () => VNode[];
        error: (props: { error: Error }) => VNode[];
      };
    };
  };
}

export default BgqlDefer;
