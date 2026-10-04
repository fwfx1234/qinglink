<script setup lang="ts">
import { computed } from 'vue'
import Spinner from './Spinner.vue'

type Variant = 'primary' | 'secondary' | 'ghost' | 'danger' | 'danger-ghost'
type Size = 'sm' | 'md'

const props = withDefaults(
  defineProps<{
    variant?: Variant
    size?: Size
    type?: 'button' | 'submit' | 'reset'
    disabled?: boolean
    loading?: boolean
    block?: boolean
    title?: string
  }>(),
  {
    variant: 'secondary',
    size: 'md',
    type: 'button',
    disabled: false,
    loading: false,
    block: false,
    title: undefined,
  },
)

const classes = computed(() => [
  'btn',
  `btn--${props.variant}`,
  `btn--${props.size}`,
  { 'btn--block': props.block },
])

const isDisabled = computed(() => props.disabled || props.loading)
</script>

<template>
  <button :type="props.type" :class="classes" :disabled="isDisabled" :title="props.title">
    <Spinner v-if="props.loading" :size="12" />
    <slot />
  </button>
</template>

<style scoped>
/* 层级：primary 近黑实心（页面上唯一的强对比锚点），secondary 白底发丝描边（默认），
   ghost 无边框（工具条里的次级动作），danger 实心红（对话框里确认破坏性动作），
   danger-ghost 只在悬停时显露红色（列表行内联的删除 / 撤销）。 */
.btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: var(--space-2);
  border: 1px solid transparent;
  border-radius: var(--radius-md);
  font-weight: var(--weight-medium);
  cursor: pointer;
  white-space: nowrap;
  transition: background var(--transition-fast), border-color var(--transition-fast),
    color var(--transition-fast), box-shadow var(--transition-fast),
    transform var(--transition-fast);
}

.btn:active:not(:disabled) {
  transform: translateY(0.5px);
}

.btn--sm {
  height: var(--control-height-sm);
  padding: 0 var(--space-3);
  font-size: var(--font-sm);
}

.btn--md {
  height: var(--control-height);
  padding: 0 var(--space-4);
  font-size: var(--font-md);
}

.btn--block {
  width: 100%;
}

.btn:disabled {
  cursor: not-allowed;
  opacity: 0.45;
}

.btn:focus-visible {
  outline: none;
  box-shadow: 0 0 0 3px var(--accent-ring);
}

.btn--primary {
  background: var(--primary-bg);
  color: var(--primary-fg);
  box-shadow: var(--shadow-xs);
}

.btn--primary:hover:not(:disabled) {
  background: var(--primary-bg-hover);
}

.btn--secondary {
  background: var(--surface-1);
  border-color: var(--border-1);
  color: var(--text-1);
  box-shadow: var(--shadow-xs);
}

.btn--secondary:hover:not(:disabled) {
  background: var(--surface-2);
  border-color: var(--border-2);
}

.btn--ghost {
  background: transparent;
  color: var(--text-2);
}

.btn--ghost:hover:not(:disabled) {
  background: var(--surface-2);
  color: var(--text-1);
}

.btn--danger {
  background: var(--error);
  color: var(--error-contrast);
  box-shadow: var(--shadow-xs);
}

.btn--danger:hover:not(:disabled) {
  filter: brightness(0.94);
}

.btn--danger-ghost {
  background: transparent;
  color: var(--text-2);
}

.btn--danger-ghost:hover:not(:disabled) {
  background: var(--error-soft);
  color: var(--error);
}
</style>
