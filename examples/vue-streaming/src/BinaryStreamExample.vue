<script setup lang="ts">
import { computed } from 'vue'
import { BgqlBinaryStream, useBinaryStream } from '@bgql/client/vue'
import type { BinaryStreamHandle } from '@bgql/client/vue'

const props = defineProps<{
  streamHandle: BinaryStreamHandle
}>()

// Use getter function to maintain reactivity when props.streamHandle changes
const { state, play, pause: pauseStream } = useBinaryStream(
  () => props.streamHandle
)

// Computed properties for template
const progress = computed(() => state.progress)
const bufferedRanges = computed(() => state.buffered)
const isPaused = computed(() => state.paused)

const togglePlayback = async () => {
  if (isPaused.value) {
    await play()
  } else {
    await pauseStream()
  }
}
</script>

<template>
  <div class="binary-stream-example">
    <h3>Binary Stream Demo</h3>

    <!-- Video Player with Progressive Loading -->
    <BgqlBinaryStream
      :stream="props.streamHandle"
      type="video"
      :progressive="true"
      class="video-player"
    />

    <!-- Playback Controls -->
    <div class="controls">
      <button @click="togglePlayback">
        {{ isPaused ? 'Play' : 'Pause' }}
      </button>

      <!-- Progress Bar -->
      <div class="progress-bar">
        <div
          class="progress-fill"
          :style="{ width: `${(progress ?? 0) * 100}%` }"
        />
        <!-- Buffered Ranges -->
        <div
          v-for="(range, i) in bufferedRanges"
          :key="i"
          class="buffered-range"
          :style="{
            left: `${range.start * 100}%`,
            width: `${(range.end - range.start) * 100}%`
          }"
        />
      </div>

      <span class="progress-text">
        {{ Math.round((progress ?? 0) * 100) }}%
      </span>
    </div>

    <!-- Error Display -->
    <div v-if="state.error" class="error">
      Error: {{ state.error.message }}
    </div>
  </div>
</template>

<style scoped>
.binary-stream-example {
  padding: 1rem;
  border: 1px solid #ddd;
  border-radius: 8px;
}

.video-player {
  width: 100%;
  max-width: 640px;
  border-radius: 4px;
}

.controls {
  display: flex;
  align-items: center;
  gap: 1rem;
  margin-top: 1rem;
}

.progress-bar {
  flex: 1;
  height: 8px;
  background: #e0e0e0;
  border-radius: 4px;
  position: relative;
  overflow: hidden;
}

.progress-fill {
  height: 100%;
  background: #2196f3;
  transition: width 0.1s;
}

.buffered-range {
  position: absolute;
  top: 0;
  height: 100%;
  background: rgba(33, 150, 243, 0.3);
}

.progress-text {
  min-width: 3rem;
  text-align: right;
  font-size: 0.875rem;
  color: #666;
}

button {
  padding: 0.5rem 1rem;
  border: none;
  border-radius: 4px;
  background: #2196f3;
  color: white;
  cursor: pointer;
}

button:hover {
  background: #1976d2;
}

.error {
  margin-top: 1rem;
  padding: 0.75rem;
  background: #ffebee;
  color: #c62828;
  border-radius: 4px;
}
</style>
