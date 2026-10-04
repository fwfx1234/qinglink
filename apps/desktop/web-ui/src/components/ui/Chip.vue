<script setup lang="ts">
import type { Tone } from '@/utils/constants'

withDefaults(
  defineProps<{
    tone?: Tone
    /** 圆点形状的指示器，用于连接状态 */
    dot?: boolean
    /** 等宽字体（级别、代码类标签） */
    mono?: boolean
    size?: 'sm' | 'md'
  }>(),
  { tone: 'neutral', dot: false, mono: false, size: 'md' },
)
</script>

<template>
  <span class="chip" :class="[`chip--${tone}`, `chip--${size}`, { 'chip--mono': mono }]">
    <span v-if="dot" class="chip__dot" aria-hidden="true" />
    <slot />
  </span>
</template>

<style scoped>
/* 标签：pill 形状 + 柔和衬底，不用边框勾勒（中性标签尤其干净）。 */
.chip {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  border-radius: var(--radius-pill);
  border: 1px solid transparent;
  font-weight: var(--weight-medium);
  white-space: nowrap;
  line-height: 1.5;
}

.chip--sm {
  padding: 0 8px;
  font-size: var(--font-xs);
  min-height: 20px;
}

.chip--md {
  padding: 2px 10px;
  font-size: var(--font-sm);
  min-height: 24px;
}

.chip--mono {
  font-family: var(--font-mono);
  letter-spacing: 0.01em;
  font-weight: var(--weight-normal);
}

.chip__dot {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  background: currentColor;
  flex: none;
}

.chip--neutral {
  background: var(--surface-2);
  color: var(--text-2);
}

.chip--accent {
  background: var(--accent-soft);
  color: var(--accent);
}

.chip--success {
  background: var(--success-soft);
  color: var(--success);
}

.chip--warning {
  background: var(--warning-soft);
  color: var(--warning);
}

.chip--error {
  background: var(--error-soft);
  color: var(--error);
}
</style>
