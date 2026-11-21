<script setup lang="ts">
withDefaults(
  defineProps<{
    size?: number
    tone?: 'accent' | 'muted'
    label?: string
  }>(),
  { size: 14, tone: 'accent', label: '加载中' },
)
</script>

<template>
  <span
    class="spinner"
    :class="`spinner--${tone}`"
    :style="{ width: `${size}px`, height: `${size}px` }"
    role="status"
    :aria-label="label"
  />
</template>

<style scoped>
.spinner {
  display: inline-block;
  border: 2px solid currentColor;
  border-top-color: transparent;
  border-radius: 50%;
  animation: spin 700ms linear infinite;
  flex: none;
}

.spinner--accent {
  color: var(--accent-bright);
}

.spinner--muted {
  color: var(--text-3);
}

@keyframes spin {
  to {
    transform: rotate(360deg);
  }
}
</style>
