<script setup lang="ts">
/**
 * 统一的错误行。SERVICE_UNAVAILABLE 不在这里显示——那是引导态，
 * 由 ServiceGate 负责（§7.5）。
 */
import type { AppError } from '@/utils/errors'

defineProps<{ error: AppError | null }>()
</script>

<template>
  <p v-if="error && !error.serviceUnavailable" class="error">
    <strong>{{ error.code }}</strong> — {{ error.message }}
  </p>
</template>

<style scoped>
.error {
  font-size: var(--font-sm);
  color: var(--error);
}
</style>
