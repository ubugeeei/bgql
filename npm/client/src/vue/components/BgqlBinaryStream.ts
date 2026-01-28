/**
 * BgqlBinaryStream Component
 *
 * Component for streaming binary content (audio/video/files).
 */

import {
  defineComponent,
  ref,
  onMounted,
  onUnmounted,
  watch,
  h,
  type PropType,
} from 'vue';
import type { BinaryStreamHandle } from '../types';
import { useBinaryStream, setupMediaSource } from '../useBinaryStream';

/**
 * BgqlBinaryStream component for media playback and file streaming.
 *
 * Supports three modes:
 * - 'audio': Renders an audio element with progressive playback
 * - 'video': Renders a video element with progressive playback
 * - 'download': Downloads the binary stream as a file
 *
 * @example
 * ```vue
 * <template>
 *   <!-- Video playback -->
 *   <BgqlBinaryStream
 *     :src="data.video.stream"
 *     type="video"
 *     :progressive="true"
 *     controls
 *   />
 *
 *   <!-- Audio playback -->
 *   <BgqlBinaryStream
 *     :src="data.audio.stream"
 *     type="audio"
 *     controls
 *   />
 *
 *   <!-- File download -->
 *   <BgqlBinaryStream
 *     :src="data.file.stream"
 *     type="download"
 *     filename="report.pdf"
 *   >
 *     <template #default="{ progress, start }">
 *       <button @click="start">Download ({{ progress }}%)</button>
 *     </template>
 *   </BgqlBinaryStream>
 * </template>
 * ```
 */
export const BgqlBinaryStream = defineComponent({
  name: 'BgqlBinaryStream',

  props: {
    /**
     * Binary stream handle from the GraphQL response.
     */
    src: {
      type: Object as PropType<BinaryStreamHandle>,
      required: true,
    },

    /**
     * Type of content to render.
     */
    type: {
      type: String as PropType<'audio' | 'video' | 'download'>,
      required: true,
    },

    /**
     * Enable progressive playback using MediaSource API.
     * Only applies to audio/video types.
     */
    progressive: {
      type: Boolean,
      default: true,
    },

    /**
     * Show native media controls.
     * Only applies to audio/video types.
     */
    controls: {
      type: Boolean,
      default: true,
    },

    /**
     * Autoplay media when loaded.
     * Only applies to audio/video types.
     */
    autoplay: {
      type: Boolean,
      default: false,
    },

    /**
     * Loop media playback.
     * Only applies to audio/video types.
     */
    loop: {
      type: Boolean,
      default: false,
    },

    /**
     * Mute audio.
     * Only applies to audio/video types.
     */
    muted: {
      type: Boolean,
      default: false,
    },

    /**
     * Poster image URL for video.
     * Only applies to video type.
     */
    poster: {
      type: String,
      default: undefined,
    },

    /**
     * Filename for download.
     * Only applies to download type.
     */
    filename: {
      type: String,
      default: 'download',
    },

    /**
     * CSS class for the media element.
     */
    mediaClass: {
      type: String,
      default: '',
    },
  },

  emits: ['play', 'pause', 'ended', 'progress', 'error'],

  setup(props, { slots, emit }) {
    const mediaRef = ref<HTMLMediaElement | null>(null);
    const { state, play, pause, seek, stop } = useBinaryStream(() => props.src);

    let mediaSourceCleanup: (() => void) | null = null;

    const setupMedia = (): void => {
      if (!mediaRef.value || !props.src) {
        return;
      }

      // Clean up previous setup
      mediaSourceCleanup?.();

      if (props.progressive && 'MediaSource' in window) {
        // Use MediaSource for progressive playback
        const { cleanup } = setupMediaSource(mediaRef.value, props.src);
        mediaSourceCleanup = cleanup;
      } else {
        // Fallback to direct URL (if available)
        // The server should provide a fallback URL
        mediaRef.value.src = `/graphql/binary/${props.src.id}`;
      }

      // Setup event listeners
      mediaRef.value.addEventListener('play', () => emit('play'));
      mediaRef.value.addEventListener('pause', () => emit('pause'));
      mediaRef.value.addEventListener('ended', () => emit('ended'));
      mediaRef.value.addEventListener('error', (e) =>
        emit('error', (e.target as HTMLMediaElement).error)
      );

      if (props.autoplay) {
        mediaRef.value.play().catch((err) => {
          console.warn('Autoplay blocked:', err);
        });
      }
    };

    onMounted(() => {
      if (props.type !== 'download') {
        setupMedia();
      }
    });

    onUnmounted(() => {
      mediaSourceCleanup?.();
      stop();
    });

    watch(
      () => props.src,
      () => {
        if (props.type !== 'download') {
          setupMedia();
        }
      }
    );

    // Watch progress for emit
    watch(
      () => state.progress,
      (progress) => {
        emit('progress', progress);
      }
    );

    return () => {
      // Download type
      if (props.type === 'download') {
        if (slots.default) {
          return slots.default({
            progress: Math.round(state.progress * 100),
            state,
            start: play,
            pause,
            stop,
          });
        }

        // Default download button
        return h(
          'button',
          {
            class: 'bgql-download-button',
            onClick: play,
            disabled: !state.paused && !state.ended,
          },
          [
            state.paused && !state.ended
              ? 'Download'
              : state.ended
              ? 'Downloaded'
              : `Downloading... ${Math.round(state.progress * 100)}%`,
          ]
        );
      }

      // Audio type
      if (props.type === 'audio') {
        return h('audio', {
          ref: mediaRef,
          class: ['bgql-audio', props.mediaClass],
          controls: props.controls,
          loop: props.loop,
          muted: props.muted,
        });
      }

      // Video type
      if (props.type === 'video') {
        return h('video', {
          ref: mediaRef,
          class: ['bgql-video', props.mediaClass],
          controls: props.controls,
          loop: props.loop,
          muted: props.muted,
          poster: props.poster,
        });
      }

      return null;
    };
  },
});

export default BgqlBinaryStream;
