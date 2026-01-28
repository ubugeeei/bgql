/**
 * BgqlStream Component
 *
 * Component for rendering @stream list items progressively.
 */

import {
  defineComponent,
  inject,
  computed,
  h,
  type PropType,
  type VNode,
  type Component,
} from 'vue';
import type { BgqlQueryContext, StreamState } from '../types';
import { BGQL_QUERY_CONTEXT_KEY } from './BgqlDefer';

/**
 * BgqlStream component for rendering streamed list items.
 *
 * This component renders list items as they arrive via @stream,
 * supporting progressive loading with placeholders.
 *
 * @example
 * ```vue
 * <template>
 *   <BgqlProvider :query="query">
 *     <BgqlStream
 *       label="posts"
 *       :path="['user', 'posts']"
 *       v-slot="{ items, hasMore, loadedCount }"
 *     >
 *       <PostCard
 *         v-for="post in items"
 *         :key="post.id"
 *         :post="post"
 *       />
 *       <LoadingCard v-if="hasMore" />
 *     </BgqlStream>
 *   </BgqlProvider>
 * </template>
 * ```
 */
export const BgqlStream = defineComponent({
  name: 'BgqlStream',

  props: {
    /**
     * The label of the streamed field.
     * Must match the label in @stream(label: "...").
     */
    label: {
      type: String,
      required: true,
    },

    /**
     * Path to the streamed array in the data.
     * e.g., ['user', 'posts'] for data.user.posts
     */
    path: {
      type: Array as PropType<string[]>,
      required: true,
    },

    /**
     * Component to render for each item.
     * Alternative to using the default slot.
     */
    itemComponent: {
      type: Object as PropType<Component>,
      default: undefined,
    },

    /**
     * Key extractor function for items.
     */
    itemKey: {
      type: Function as PropType<(item: unknown, index: number) => string | number>,
      default: (item: unknown, index: number) => {
        if (typeof item === 'object' && item !== null && 'id' in item) {
          return (item as { id: string | number }).id;
        }
        return index;
      },
    },

    /**
     * Number of placeholder items to show while loading.
     */
    placeholderCount: {
      type: Number,
      default: 0,
    },
  },

  setup(props, { slots }) {
    const context = inject<BgqlQueryContext>(BGQL_QUERY_CONTEXT_KEY);

    if (!context) {
      console.warn(
        'BgqlStream must be used within a BgqlProvider or component that provides bgql-query-context'
      );
    }

    const items = computed<unknown[]>(() => {
      if (!context?.data) {
        return [];
      }

      let current: unknown = context.data;
      for (const key of props.path) {
        if (current && typeof current === 'object' && key in current) {
          current = (current as Record<string, unknown>)[key];
        } else {
          return [];
        }
      }

      return Array.isArray(current) ? current : [];
    });

    const hasMore = computed(() => {
      if (!context) {
        return false;
      }

      const streamState = context.streamState as StreamState;
      return (
        streamState.hasNext ||
        streamState.activeStreams.includes(props.label)
      );
    });

    const loadedCount = computed(() => items.value.length);

    const isLoading = computed(() => {
      return context?.loading ?? false;
    });

    return () => {
      // Use default slot if provided
      if (slots.default) {
        return slots.default({
          items: items.value,
          hasMore: hasMore.value,
          loadedCount: loadedCount.value,
          isLoading: isLoading.value,
        });
      }

      // Use itemComponent if provided
      if (props.itemComponent) {
        const renderedItems = items.value.map((item, index) =>
          h(props.itemComponent!, {
            key: props.itemKey(item, index),
            item,
            index,
          })
        );

        // Add placeholders
        if (hasMore.value && props.placeholderCount > 0 && slots.placeholder) {
          for (let i = 0; i < props.placeholderCount; i++) {
            renderedItems.push(
              h('div', { key: `placeholder-${i}` }, slots.placeholder())
            );
          }
        }

        return h('div', { class: 'bgql-stream' }, renderedItems);
      }

      // No rendering method provided
      console.warn(
        'BgqlStream requires either a default slot or itemComponent prop'
      );
      return null;
    };
  },
});

/**
 * Type-safe version of BgqlStream for TypeScript.
 */
export function createTypedStream<TItem>() {
  return BgqlStream as typeof BgqlStream & {
    new (): {
      $slots: {
        default: (props: {
          items: TItem[];
          hasMore: boolean;
          loadedCount: number;
          isLoading: boolean;
        }) => VNode[];
        placeholder: () => VNode[];
      };
    };
  };
}

export default BgqlStream;
