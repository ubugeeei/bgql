/**
 * useBinaryStream Composable for BGQL
 *
 * Vue composable for handling binary streaming (audio/video/files).
 */

import {
  ref,
  onUnmounted,
  watch,
} from 'vue';
import type {
  BinaryStreamHandle,
  BinaryStreamState,
  BufferedRange,
  UseBinaryStreamResult,
} from './types';

/**
 * Binary protocol frame flags.
 */
const FRAME_FLAGS = {
  FINAL: 0x01,
  ERROR: 0x02,
  METADATA: 0x04,
} as const;

/**
 * Default binary stream state.
 */
const DEFAULT_STATE: BinaryStreamState = {
  position: 0,
  buffered: [],
  paused: true,
  ended: false,
  error: null,
  progress: 0,
};

/**
 * Handles binary streaming for media and file content.
 *
 * This composable manages the binary streaming protocol defined in
 * bgql_runtime::binary_transport, providing:
 * - Progressive download/streaming
 * - Pause/resume support
 * - Seek within buffered ranges
 * - Progress tracking
 *
 * @example
 * ```vue
 * <script setup>
 * import { useBinaryStream } from '@bgql/client/vue'
 *
 * const props = defineProps<{ stream: BinaryStreamHandle }>()
 *
 * const { state, play, pause, seek } = useBinaryStream(
 *   () => props.stream
 * )
 * </script>
 *
 * <template>
 *   <div>
 *     <progress :value="state.progress" max="1" />
 *     <button @click="state.paused ? play() : pause()">
 *       {{ state.paused ? 'Play' : 'Pause' }}
 *     </button>
 *   </div>
 * </template>
 * ```
 */
export function useBinaryStream(
  handle: BinaryStreamHandle | (() => BinaryStreamHandle | null)
): UseBinaryStreamResult {
  const state = ref<BinaryStreamState>({ ...DEFAULT_STATE });
  const chunks = ref<ArrayBuffer[]>([]);

  let abortController: AbortController | null = null;
  let reader: ReadableStreamDefaultReader<Uint8Array> | null = null;

  const getHandle = (): BinaryStreamHandle | null => {
    return typeof handle === 'function' ? handle() : handle;
  };

  const startStream = async (): Promise<void> => {
    const streamHandle = getHandle();
    if (!streamHandle) {
      return;
    }

    abortController?.abort();
    abortController = new AbortController();

    state.value = {
      ...DEFAULT_STATE,
      paused: false,
    };

    try {
      const response = await fetch(getBinaryStreamUrl(streamHandle.id), {
        method: 'GET',
        headers: {
          Accept: 'application/vnd.bgql.binary-stream',
          'X-BGQL-Stream-Id': streamHandle.id,
        },
        signal: abortController.signal,
      });

      if (!response.ok) {
        throw new Error(`HTTP ${response.status}: ${response.statusText}`);
      }

      if (!response.body) {
        throw new Error('Response body is not available');
      }

      reader = response.body.getReader();
      await processStream(streamHandle);
    } catch (err) {
      if (err instanceof Error && err.name !== 'AbortError') {
        state.value = {
          ...state.value,
          error: err,
          paused: true,
        };
      }
    }
  };

  const processStream = async (
    streamHandle: BinaryStreamHandle
  ): Promise<void> => {
    if (!reader) {
      return;
    }

    let totalBytesReceived = 0;
    let buffer = new ArrayBuffer(0);
    let bufferView = new Uint8Array(buffer);

    try {
      while (!state.value.paused && !state.value.ended) {
        const { done, value } = await reader.read();

        if (done) {
          state.value = {
            ...state.value,
            ended: true,
            progress: 1,
          };
          break;
        }

        // Append to buffer
        const newBuffer = new ArrayBuffer(bufferView.length + value.length);
        const newView = new Uint8Array(newBuffer);
        newView.set(bufferView, 0);
        newView.set(value, bufferView.length);
        buffer = newBuffer;
        bufferView = newView;

        // Process complete frames
        while (bufferView.length >= 13) { // Minimum frame size: 4 + 4 + 1 + 0 + 4
          const frame = parseFrame(bufferView);
          if (!frame) {
            break;
          }

          // Slice buffer
          buffer = buffer.slice(frame.totalLength);
          bufferView = new Uint8Array(buffer);
          totalBytesReceived += frame.payload.byteLength;

          // Store chunk
          chunks.value.push(frame.payload);

          // Update state
          const progress = streamHandle.totalSize
            ? totalBytesReceived / streamHandle.totalSize
            : null;

          state.value = {
            ...state.value,
            position: totalBytesReceived,
            progress: progress ?? state.value.progress,
            buffered: calculateBufferedRanges(chunks.value),
            ended: (frame.flags & FRAME_FLAGS.FINAL) !== 0,
          };

          if (frame.flags & FRAME_FLAGS.ERROR) {
            const errorMessage = new TextDecoder().decode(frame.payload);
            throw new Error(errorMessage);
          }
        }
      }
    } finally {
      reader.releaseLock();
      reader = null;
    }
  };

  const play = (): void => {
    if (state.value.ended) {
      // Restart from beginning
      chunks.value = [];
      startStream();
    } else if (state.value.paused) {
      state.value = { ...state.value, paused: false };
      startStream();
    }
  };

  const pause = (): void => {
    state.value = { ...state.value, paused: true };
    abortController?.abort();
  };

  const seek = (position: number): void => {
    const streamHandle = getHandle();
    if (!streamHandle?.supportsRange) {
      console.warn('Stream does not support seeking');
      return;
    }

    // Check if position is within buffered ranges
    const isBuffered = state.value.buffered.some(
      (range) => position >= range.start && position <= range.end
    );

    if (isBuffered) {
      state.value = { ...state.value, position };
    } else {
      // Need to fetch from new position
      chunks.value = [];
      state.value = {
        ...DEFAULT_STATE,
        position,
        paused: false,
      };
      startStreamFromPosition(position);
    }
  };

  const startStreamFromPosition = async (position: number): Promise<void> => {
    const streamHandle = getHandle();
    if (!streamHandle) {
      return;
    }

    abortController?.abort();
    abortController = new AbortController();

    try {
      const response = await fetch(getBinaryStreamUrl(streamHandle.id), {
        method: 'GET',
        headers: {
          Accept: 'application/vnd.bgql.binary-stream',
          'X-BGQL-Stream-Id': streamHandle.id,
          Range: `bytes=${position}-`,
        },
        signal: abortController.signal,
      });

      if (!response.ok && response.status !== 206) {
        throw new Error(`HTTP ${response.status}: ${response.statusText}`);
      }

      if (!response.body) {
        throw new Error('Response body is not available');
      }

      reader = response.body.getReader();
      await processStream(streamHandle);
    } catch (err) {
      if (err instanceof Error && err.name !== 'AbortError') {
        state.value = {
          ...state.value,
          error: err,
          paused: true,
        };
      }
    }
  };

  const stop = (): void => {
    abortController?.abort();
    reader?.cancel();
    reader = null;
    chunks.value = [];
    state.value = { ...DEFAULT_STATE };
  };

  // Watch for handle changes
  watch(
    () => getHandle(),
    (newHandle) => {
      if (newHandle) {
        stop();
      }
    }
  );

  // Cleanup
  onUnmounted(() => {
    stop();
  });

  return {
    get state() {
      return state.value;
    },
    play,
    pause,
    seek,
    stop,
  };
}

// =============================================================================
// MediaSource Integration
// =============================================================================

/**
 * Sets up MediaSource API for progressive media playback.
 *
 * @example
 * ```vue
 * <script setup>
 * import { ref, onMounted } from 'vue'
 * import { setupMediaSource } from '@bgql/client/vue'
 *
 * const videoRef = ref<HTMLVideoElement>()
 *
 * onMounted(() => {
 *   setupMediaSource(videoRef.value!, props.stream)
 * })
 * </script>
 *
 * <template>
 *   <video ref="videoRef" controls />
 * </template>
 * ```
 */
export function setupMediaSource(
  element: HTMLMediaElement,
  handle: BinaryStreamHandle
): { cleanup: () => void } {
  if (!('MediaSource' in window)) {
    console.warn('MediaSource API not available');
    return { cleanup: () => {} };
  }

  const mediaSource = new MediaSource();
  element.src = URL.createObjectURL(mediaSource);

  let sourceBuffer: SourceBuffer | null = null;
  let abortController: AbortController | null = null;
  const pendingChunks: ArrayBuffer[] = [];

  const appendNextChunk = (): void => {
    if (!sourceBuffer || sourceBuffer.updating || pendingChunks.length === 0) {
      return;
    }

    const chunk = pendingChunks.shift()!;
    sourceBuffer.appendBuffer(chunk);
  };

  mediaSource.addEventListener('sourceopen', async () => {
    try {
      // Determine MIME type from content type
      const mimeType = getMimeType(handle.contentType);
      if (!MediaSource.isTypeSupported(mimeType)) {
        throw new Error(`Unsupported MIME type: ${mimeType}`);
      }

      sourceBuffer = mediaSource.addSourceBuffer(mimeType);
      sourceBuffer.addEventListener('updateend', appendNextChunk);

      // Start fetching stream
      abortController = new AbortController();

      const response = await fetch(getBinaryStreamUrl(handle.id), {
        method: 'GET',
        headers: {
          Accept: 'application/vnd.bgql.binary-stream',
          'X-BGQL-Stream-Id': handle.id,
        },
        signal: abortController.signal,
      });

      if (!response.ok || !response.body) {
        throw new Error(`Failed to fetch stream: ${response.status}`);
      }

      const reader = response.body.getReader();
      let buffer = new ArrayBuffer(0);
      let bufferView = new Uint8Array(buffer);

      while (true) {
        const { done, value } = await reader.read();

        if (done) {
          if (mediaSource.readyState === 'open') {
            mediaSource.endOfStream();
          }
          break;
        }

        // Append to buffer
        const newBuffer = new ArrayBuffer(bufferView.length + value.length);
        const newView = new Uint8Array(newBuffer);
        newView.set(bufferView, 0);
        newView.set(value, bufferView.length);
        buffer = newBuffer;
        bufferView = newView;

        // Process complete frames
        while (bufferView.length >= 13) {
          const frame = parseFrame(bufferView);
          if (!frame) {
            break;
          }

          buffer = buffer.slice(frame.totalLength);
          bufferView = new Uint8Array(buffer);
          pendingChunks.push(frame.payload);
          appendNextChunk();

          if (frame.flags & FRAME_FLAGS.FINAL) {
            if (mediaSource.readyState === 'open') {
              // Wait for all pending chunks to be appended
              await new Promise<void>((resolve) => {
                const checkPending = (): void => {
                  if (pendingChunks.length === 0 && !sourceBuffer?.updating) {
                    resolve();
                  } else {
                    setTimeout(checkPending, 100);
                  }
                };
                checkPending();
              });
              mediaSource.endOfStream();
            }
            break;
          }
        }
      }
    } catch (err) {
      console.error('MediaSource error:', err);
      if (mediaSource.readyState === 'open') {
        mediaSource.endOfStream('decode');
      }
    }
  });

  const cleanup = (): void => {
    abortController?.abort();
    URL.revokeObjectURL(element.src);
    element.src = '';
  };

  return { cleanup };
}

// =============================================================================
// Helper Functions
// =============================================================================

function getBinaryStreamUrl(streamId: string): string {
  if (typeof window !== 'undefined' && (window as unknown as { __BGQL_ENDPOINT__?: string }).__BGQL_ENDPOINT__) {
    const base = (window as unknown as { __BGQL_ENDPOINT__?: string }).__BGQL_ENDPOINT__!;
    return `${base}/binary/${streamId}`;
  }
  return `/graphql/binary/${streamId}`;
}

interface ParsedFrame {
  sequence: number;
  flags: number;
  payload: ArrayBuffer;
  totalLength: number;
}

function parseFrame(bufferView: Uint8Array): ParsedFrame | null {
  if (bufferView.length < 13) {
    return null;
  }

  const view = new DataView(bufferView.buffer, bufferView.byteOffset, bufferView.length);

  const sequence = view.getUint32(0, false); // Big-endian
  const payloadLength = view.getUint32(4, false);
  const flags = view.getUint8(8);

  const frameHeaderSize = 9;
  const crcSize = 4;
  const totalLength = frameHeaderSize + payloadLength + crcSize;

  if (bufferView.length < totalLength) {
    return null; // Incomplete frame
  }

  // Copy payload to a new ArrayBuffer
  const payload = bufferView.slice(frameHeaderSize, frameHeaderSize + payloadLength).buffer.slice(0);

  // TODO: Verify CRC32

  return {
    sequence,
    flags,
    payload,
    totalLength,
  };
}

function calculateBufferedRanges(chunks: ArrayBuffer[]): BufferedRange[] {
  if (chunks.length === 0) {
    return [];
  }

  let start = 0;
  let end = 0;

  for (const chunk of chunks) {
    end += chunk.byteLength;
  }

  return [{ start, end }];
}

function getMimeType(contentType: string): string {
  // Map common content types to MediaSource-compatible MIME types
  const mimeMap: Record<string, string> = {
    'video/mp4': 'video/mp4; codecs="avc1.42E01E, mp4a.40.2"',
    'video/webm': 'video/webm; codecs="vp8, vorbis"',
    'audio/mp4': 'audio/mp4; codecs="mp4a.40.2"',
    'audio/webm': 'audio/webm; codecs="vorbis"',
    'audio/mpeg': 'audio/mpeg',
  };

  return mimeMap[contentType] ?? contentType;
}

/**
 * Creates a Blob URL from streamed chunks for download.
 */
export function createBlobUrl(
  chunks: ArrayBuffer[],
  contentType: string
): string {
  const blob = new Blob(chunks, { type: contentType });
  return URL.createObjectURL(blob);
}

/**
 * Downloads streamed content as a file.
 */
export function downloadBinaryStream(
  chunks: ArrayBuffer[],
  contentType: string,
  filename: string
): void {
  const url = createBlobUrl(chunks, contentType);
  const a = document.createElement('a');
  a.href = url;
  a.download = filename;
  a.click();
  URL.revokeObjectURL(url);
}
