<script setup lang="ts">
defineProps<{
  errors: number
  isProcessing: boolean
  version: string
}>()
</script>

<template>
  <footer class="status-bar">
    <div class="status-bar__left">
      <div :class="['status-bar__item', errors > 0 ? 'error' : 'success']">
        <i :class="['mdi', errors > 0 ? 'mdi-alert-circle' : 'mdi-check-circle']"></i>
        {{ errors > 0 ? `${errors} error(s)` : 'No issues' }}
      </div>

      <div v-if="isProcessing" class="status-bar__item">
        <div class="spinner" style="width: 12px; height: 12px; border-width: 1.5px;"></div>
        Processing...
      </div>
    </div>

    <div class="status-bar__right">
      <div class="status-bar__item">
        <i class="mdi mdi-code-braces"></i>
        bgql
      </div>

      <div class="status-bar__item">
        <i class="mdi mdi-information-outline"></i>
        v{{ version }}
      </div>
    </div>
  </footer>
</template>
