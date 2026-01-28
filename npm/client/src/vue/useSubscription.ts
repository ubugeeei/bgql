/**
 * useSubscription Composable for BGQL
 *
 * Vue composable for GraphQL subscriptions via WebSocket.
 *
 * NOTE: This is a placeholder for future implementation.
 * Full WebSocket subscription support is planned for a future release.
 */

import {
  ref,
  inject,
  onUnmounted,
  toValue,
  type Ref,
  type MaybeRefOrGetter,
} from 'vue';
import type { DocumentNode, RequestOptions } from './types';
import type { BgqlClient } from '../client';
import { BGQL_CLIENT_KEY } from './useQuery';

/**
 * Subscription options.
 */
export interface SubscriptionOptions<TVariables = Record<string, unknown>> {
  /**
   * Variables for the subscription.
   */
  readonly variables?: MaybeRefOrGetter<TVariables | undefined>;

  /**
   * Skip/pause the subscription.
   */
  readonly skip?: MaybeRefOrGetter<boolean>;

  /**
   * Custom client instance. If not provided, uses injected client.
   */
  readonly client?: BgqlClient;

  /**
   * Request options.
   */
  readonly requestOptions?: RequestOptions;

  /**
   * Callback when data is received.
   */
  readonly onData?: (data: unknown) => void;

  /**
   * Callback when subscription completes.
   */
  readonly onComplete?: () => void;

  /**
   * Callback when an error occurs.
   */
  readonly onError?: (error: Error) => void;

  /**
   * Whether to automatically reconnect on connection loss.
   * @default true
   */
  readonly reconnect?: boolean;

  /**
   * Reconnection attempts before giving up.
   * @default 5
   */
  readonly reconnectAttempts?: number;

  /**
   * Delay between reconnection attempts in milliseconds.
   * @default 3000
   */
  readonly reconnectDelay?: number;
}

/**
 * Subscription state.
 */
export type SubscriptionStatus =
  | 'idle'
  | 'connecting'
  | 'connected'
  | 'reconnecting'
  | 'error'
  | 'closed';

/**
 * Result of useSubscription composable.
 */
export interface UseSubscriptionResult<TData> {
  /**
   * The latest data received from the subscription.
   */
  readonly data: TData | null;

  /**
   * Error from the subscription.
   */
  readonly error: Error | null;

  /**
   * Current subscription status.
   */
  readonly status: SubscriptionStatus;

  /**
   * Whether the subscription is currently active.
   */
  readonly isActive: boolean;

  /**
   * Start/restart the subscription.
   */
  readonly start: () => void;

  /**
   * Stop the subscription.
   */
  readonly stop: () => void;

  /**
   * Restart the subscription.
   */
  readonly restart: () => void;
}

/**
 * Subscribe to a GraphQL subscription via WebSocket.
 *
 * NOTE: This is a placeholder implementation. Full WebSocket support
 * is planned for a future release.
 *
 * @example
 * ```vue
 * <script setup>
 * import { useSubscription } from '@bgql/client/vue'
 *
 * const { data, error, status, start, stop } = useSubscription(
 *   gql`
 *     subscription OnNewMessage($roomId: ID!) {
 *       newMessage(roomId: $roomId) {
 *         id
 *         content
 *         author {
 *           id
 *           name
 *         }
 *         createdAt
 *       }
 *     }
 *   `,
 *   {
 *     variables: () => ({ roomId: props.roomId }),
 *     onData: (message) => {
 *       messages.value.push(message.newMessage)
 *     },
 *   }
 * )
 * </script>
 *
 * <template>
 *   <div>
 *     <p>Status: {{ status }}</p>
 *     <button @click="status === 'connected' ? stop() : start()">
 *       {{ status === 'connected' ? 'Disconnect' : 'Connect' }}
 *     </button>
 *     <div v-if="error" class="error">{{ error.message }}</div>
 *   </div>
 * </template>
 * ```
 */
export function useSubscription<TData = unknown, TVariables = Record<string, unknown>>(
  subscription: DocumentNode | string,
  options: SubscriptionOptions<TVariables> = {}
): UseSubscriptionResult<TData> {
  // Get the injected client
  const injectedClient = inject<BgqlClient | null>(BGQL_CLIENT_KEY, null);
  const client = options.client ?? injectedClient;

  const data = ref<TData | null>(null) as Ref<TData | null>;
  const error = ref<Error | null>(null);
  const status = ref<SubscriptionStatus>('idle');

  // WebSocket connection (placeholder - to be implemented)
  let ws: WebSocket | null = null;
  let reconnectAttempts = 0;
  const maxReconnectAttempts = options.reconnectAttempts ?? 5;
  const reconnectDelay = options.reconnectDelay ?? 3000;
  const shouldReconnect = options.reconnect !== false;

  /**
   * Get the WebSocket URL.
   */
  const getWsUrl = (): string => {
    if (typeof window === 'undefined') {
      return 'ws://localhost:4000/graphql/ws';
    }

    const wsEndpoint = (window as unknown as { __BGQL_WS_ENDPOINT__?: string }).__BGQL_WS_ENDPOINT__;
    if (wsEndpoint) {
      const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
      return `${protocol}//${window.location.host}${wsEndpoint}`;
    }

    const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
    return `${protocol}//${window.location.host}/graphql/ws`;
  };

  /**
   * Start the subscription.
   */
  const start = (): void => {
    if (toValue(options.skip)) {
      return;
    }

    if (ws && ws.readyState === WebSocket.OPEN) {
      return; // Already connected
    }

    status.value = 'connecting';
    error.value = null;

    try {
      ws = new WebSocket(getWsUrl(), 'graphql-ws');

      ws.onopen = () => {
        status.value = 'connected';
        reconnectAttempts = 0;

        // Send connection init
        ws!.send(JSON.stringify({ type: 'connection_init' }));

        // Send subscription
        const subscriptionString = typeof subscription === 'string'
          ? subscription
          : getSubscriptionString(subscription);

        ws!.send(JSON.stringify({
          id: '1',
          type: 'subscribe',
          payload: {
            query: subscriptionString,
            variables: toValue(options.variables),
          },
        }));
      };

      ws.onmessage = (event) => {
        try {
          const message = JSON.parse(event.data);

          switch (message.type) {
            case 'next':
              if (message.payload?.data) {
                data.value = message.payload.data;
                options.onData?.(message.payload.data);
              }
              break;

            case 'error':
              const err = new Error(message.payload?.message ?? 'Subscription error');
              error.value = err;
              options.onError?.(err);
              break;

            case 'complete':
              status.value = 'closed';
              options.onComplete?.();
              break;
          }
        } catch (err) {
          console.error('[BGQL] Failed to parse subscription message:', err);
        }
      };

      ws.onerror = (event) => {
        const err = new Error('WebSocket error');
        error.value = err;
        status.value = 'error';
        options.onError?.(err);
      };

      ws.onclose = () => {
        status.value = 'closed';

        // Attempt reconnection if enabled
        if (shouldReconnect && reconnectAttempts < maxReconnectAttempts) {
          status.value = 'reconnecting';
          reconnectAttempts++;
          setTimeout(start, reconnectDelay);
        }
      };
    } catch (err) {
      const connectError = err instanceof Error ? err : new Error(String(err));
      error.value = connectError;
      status.value = 'error';
      options.onError?.(connectError);
    }
  };

  /**
   * Stop the subscription.
   */
  const stop = (): void => {
    if (ws) {
      // Send unsubscribe message
      if (ws.readyState === WebSocket.OPEN) {
        ws.send(JSON.stringify({ id: '1', type: 'complete' }));
      }
      ws.close();
      ws = null;
    }
    status.value = 'closed';
    reconnectAttempts = maxReconnectAttempts; // Prevent auto-reconnect
  };

  /**
   * Restart the subscription.
   */
  const restart = (): void => {
    stop();
    reconnectAttempts = 0;
    start();
  };

  // Cleanup on unmount
  onUnmounted(() => {
    stop();
  });

  // Note: Auto-start is intentionally not implemented
  // Users should call start() explicitly

  return {
    get data() {
      return data.value;
    },
    get error() {
      return error.value;
    },
    get status() {
      return status.value;
    },
    get isActive() {
      return status.value === 'connected' || status.value === 'connecting' || status.value === 'reconnecting';
    },
    start,
    stop,
    restart,
  };
}

// =============================================================================
// Helper Functions
// =============================================================================

function getSubscriptionString(doc: DocumentNode): string {
  const source = (doc as unknown as { loc?: { source?: { body?: string } } }).loc?.source?.body;
  if (source) {
    return source;
  }
  throw new Error('Cannot extract subscription string from DocumentNode');
}

export default useSubscription;
