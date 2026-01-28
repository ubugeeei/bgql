/**
 * BgqlProvider Component
 *
 * Provides BGQL query context to child components.
 */

import {
  defineComponent,
  provide,
  computed,
  h,
  type PropType,
} from 'vue';
import type {
  DocumentNode,
  QueryOptions,
  BgqlQueryContext,
} from '../types';
import { useQuery } from '../useQuery';
import { BGQL_QUERY_CONTEXT_KEY } from './BgqlDefer';

/**
 * BgqlProvider component that executes a query and provides context to children.
 *
 * This component combines useQuery with context provision, allowing child
 * components like BgqlDefer and BgqlStream to access query state.
 *
 * @example
 * ```vue
 * <template>
 *   <BgqlProvider
 *     :query="GetUserQuery"
 *     :variables="{ id: userId }"
 *     streaming
 *     v-slot="{ data, loading, error }"
 *   >
 *     <div v-if="loading && !data">Loading...</div>
 *     <div v-else-if="error">Error: {{ error.message }}</div>
 *     <div v-else>
 *       <h1>{{ data.user.name }}</h1>
 *       <BgqlDefer label="bio">
 *         <template #default>
 *           <p>{{ data.user.bio }}</p>
 *         </template>
 *         <template #fallback>
 *           <Skeleton />
 *         </template>
 *       </BgqlDefer>
 *     </div>
 *   </BgqlProvider>
 * </template>
 * ```
 */
export const BgqlProvider = defineComponent({
  name: 'BgqlProvider',

  props: {
    /**
     * The GraphQL query document or string.
     */
    query: {
      type: [Object, String] as PropType<DocumentNode | string>,
      required: true,
    },

    /**
     * Variables for the query.
     */
    variables: {
      type: Object as PropType<Record<string, unknown>>,
      default: undefined,
    },

    /**
     * Skip query execution.
     */
    skip: {
      type: Boolean,
      default: false,
    },

    /**
     * Polling interval in milliseconds.
     */
    pollInterval: {
      type: Number,
      default: undefined,
    },

    /**
     * Enable streaming (@defer/@stream) support.
     */
    streaming: {
      type: Boolean,
      default: false,
    },

    /**
     * Fetch policy.
     */
    fetchPolicy: {
      type: String as PropType<QueryOptions['fetchPolicy']>,
      default: 'cache-first',
    },
  },

  emits: ['data', 'error', 'complete'],

  setup(props, { slots, emit }) {
    const queryResult = useQuery(props.query, {
      variables: props.variables,
      skip: props.skip,
      pollInterval: props.pollInterval,
      streaming: props.streaming,
      fetchPolicy: props.fetchPolicy,
      onData: (data) => emit('data', data),
      onError: (error) => emit('error', error),
      onComplete: () => emit('complete'),
    });

    // Provide context to child components
    const context = computed<BgqlQueryContext>(() => ({
      data: queryResult.data,
      loading: queryResult.loading,
      error: queryResult.error,
      streamState: queryResult.streamState,
      controller: queryResult.controller,
    }));

    provide(BGQL_QUERY_CONTEXT_KEY, context.value);

    return () => {
      if (slots.default) {
        return slots.default({
          data: queryResult.data,
          loading: queryResult.loading,
          error: queryResult.error,
          streamState: queryResult.streamState,
          pause: queryResult.pause,
          resume: queryResult.resume,
          refetch: queryResult.refetch,
        });
      }

      return null;
    };
  },
});

export default BgqlProvider;
