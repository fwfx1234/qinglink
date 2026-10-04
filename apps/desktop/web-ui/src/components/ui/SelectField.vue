<script setup lang="ts">
import { useId } from 'vue'

export interface SelectOption {
  value: string
  label: string
}

const props = withDefaults(
  defineProps<{
    label?: string
    modelValue: string
    options: SelectOption[]
    disabled?: boolean
    /** 紧凑模式：无标签，仅用于工具条 */
    inline?: boolean
    width?: number | null
  }>(),
  { label: undefined, disabled: false, inline: false, width: null },
)

const emit = defineEmits<{
  (event: 'update:modelValue', value: string): void
}>()

const selectId = useId()

function onChange(event: Event): void {
  emit('update:modelValue', (event.target as HTMLSelectElement).value)
}
</script>

<template>
  <div class="select" :class="{ 'select--inline': props.inline }">
    <label v-if="props.label" class="select__label" :for="selectId">{{ props.label }}</label>
    <select
      :id="selectId"
      class="select__control"
      :value="props.modelValue"
      :disabled="props.disabled"
      :style="props.width ? { width: `${props.width}px` } : undefined"
      @change="onChange"
    >
      <option v-for="option in props.options" :key="option.value" :value="option.value">
        {{ option.label }}
      </option>
    </select>
  </div>
</template>

<style scoped>
.select {
  position: relative;
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
  min-width: 0;
}

.select--inline {
  gap: 0;
}

.select__label {
  font-size: var(--font-sm);
  font-weight: var(--weight-medium);
  color: var(--text-2);
}

.select__control {
  height: var(--control-height);
  padding: 0 30px 0 var(--space-3);
  border-radius: var(--radius-md);
  border: 1px solid var(--border-1);
  background: var(--surface-1);
  color: var(--text-1);
  cursor: pointer;
  outline: none;
  /* 原生箭头在各浏览器里样式不一致且偏旧；换成自绘的 mask 箭头 */
  appearance: none;
  transition: border-color var(--transition-fast), box-shadow var(--transition-fast),
    background var(--transition-fast);
}

.select__control:hover:not(:disabled) {
  border-color: var(--border-2);
}

.select__control:focus {
  border-color: var(--accent);
  box-shadow: 0 0 0 3px var(--accent-ring);
}

.select__control:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.select__control option {
  background: var(--surface-1);
  color: var(--text-1);
}

/* 箭头用 mask 绘制：形状来自蒙版，颜色仍走令牌（组件里不出现颜色字面量）。 */
.select::after {
  content: '';
  position: absolute;
  right: 12px;
  bottom: 14px;
  width: 9px;
  height: 6px;
  background: var(--text-3);
  -webkit-mask: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 9 6'%3E%3Cpath d='M1 1.5 4.5 4.5 8 1.5' fill='none' stroke='black' stroke-width='1.6' stroke-linecap='round' stroke-linejoin='round'/%3E%3C/svg%3E")
    center / contain no-repeat;
  mask: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 9 6'%3E%3Cpath d='M1 1.5 4.5 4.5 8 1.5' fill='none' stroke='black' stroke-width='1.6' stroke-linecap='round' stroke-linejoin='round'/%3E%3C/svg%3E")
    center / contain no-repeat;
  pointer-events: none;
}

.select--inline::after {
  bottom: 15px;
}
</style>
