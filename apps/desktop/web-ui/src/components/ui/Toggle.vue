<script setup lang="ts">
const props = withDefaults(
  defineProps<{
    modelValue: boolean
    disabled?: boolean
    loading?: boolean
    label: string
    /** 开关旁的补充说明文字 */
    description?: string
  }>(),
  { disabled: false, loading: false, description: undefined },
)

const emit = defineEmits<{
  (event: 'update:modelValue', value: boolean): void
}>()

function onChange(event: Event): void {
  const target = event.target as HTMLInputElement
  emit('update:modelValue', target.checked)
}
</script>

<template>
  <label class="toggle" :class="{ 'toggle--disabled': props.disabled || props.loading }">
    <span class="toggle__text">
      <span class="toggle__label">{{ props.label }}</span>
      <span v-if="props.description" class="toggle__desc">{{ props.description }}</span>
    </span>
    <span class="toggle__control">
      <input
        class="toggle__input"
        type="checkbox"
        :checked="props.modelValue"
        :disabled="props.disabled || props.loading"
        @change="onChange"
      />
      <span class="toggle__track" aria-hidden="true">
        <span class="toggle__thumb" />
      </span>
    </span>
  </label>
</template>

<style scoped>
.toggle {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--space-5);
  padding: var(--space-3) var(--space-3);
  border-radius: var(--radius-md);
  cursor: pointer;
  transition: background var(--transition-fast);
}

.toggle:hover {
  background: var(--surface-2);
}

.toggle--disabled {
  cursor: not-allowed;
  opacity: 0.5;
}

.toggle__text {
  display: flex;
  flex-direction: column;
  gap: 2px;
  min-width: 0;
}

.toggle__label {
  font-size: var(--font-md);
  font-weight: var(--weight-medium);
  color: var(--text-1);
}

.toggle__desc {
  font-size: var(--font-sm);
  color: var(--text-3);
}

.toggle__control {
  position: relative;
  flex: none;
}

.toggle__input {
  position: absolute;
  inset: 0;
  width: 100%;
  height: 100%;
  margin: 0;
  opacity: 0;
  cursor: inherit;
}

/* 开启态是实心强调色（不是"浅底 + 描边"），关闭态是中性填充：状态一眼可辨。 */
.toggle__track {
  display: block;
  width: 40px;
  height: 22px;
  border-radius: var(--radius-pill);
  background: var(--border-2);
  transition: background var(--transition-base);
}

.toggle__thumb {
  display: block;
  width: 18px;
  height: 18px;
  margin: 2px;
  border-radius: 50%;
  background: var(--surface-1);
  box-shadow: var(--shadow-xs);
  transition: transform var(--transition-base), background var(--transition-base);
}

.toggle__input:checked + .toggle__track {
  background: var(--accent);
}

.toggle__input:checked + .toggle__track .toggle__thumb {
  transform: translateX(18px);
  background: var(--accent-contrast);
}

.toggle__input:focus-visible + .toggle__track {
  box-shadow: 0 0 0 3px var(--accent-ring);
}
</style>
