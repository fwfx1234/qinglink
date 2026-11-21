<script setup lang="ts">
import type { Tone } from '@/utils/constants'

withDefaults(
  defineProps<{
    tone?: Tone
    /** 呼吸动效，用于「连接中」等过渡态 */
    pulse?: boolean
    size?: number
    label?: string
  }>(),
  { tone: 'neutral', pulse: false, size: 8, label: undefined },
)
</script>

<template>
  <span
    class="dot"
    :class="[`dot--${tone}`, { 'dot--pulse': pulse }]"
    :style="{ width: `${size}px`, height: `${size}px` }"
    role="img"
    :aria-label="label"
    :title="label"
  />
</template>

<style scoped>
.dot {
  display: inline-block;
  border-radius: 50%;
  flex: none;
}

.dot--neutral {
  background: var(--text-3);
}

.dot--accent {
  background: var(--accent-bright);
}

.dot--success {
  background: var(--success);
}

.dot--warning {
  background: var(--warning);
}

.dot--error {
  background: var(--error);
}

.dot--pulse {
  animation: pulse 1.4s ease-in-out infinite;
}

@keyframes pulse {
  0%,
  100% {
    opacity: 1;
  }
  50% {
    opacity: 0.35;
  }
}
</style>
