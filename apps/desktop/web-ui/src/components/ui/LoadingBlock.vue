<script setup lang="ts">
/** 统一的加载占位：此前在应用/设备/日志三处各写了一遍。 */
import Spinner from './Spinner.vue'

withDefaults(defineProps<{ label?: string; size?: number }>(), {
  label: '正在加载…',
  size: 16,
})
</script>

<template>
  <div class="loading">
    <Spinner :size="size" />
    <span>{{ label }}</span>
  </div>
</template>

<style scoped>
.loading {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: var(--space-3);
  padding: var(--space-8);
  color: var(--text-2);
  font-size: var(--font-sm);
}
</style>
