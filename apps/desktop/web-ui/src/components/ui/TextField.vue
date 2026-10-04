<script setup lang="ts">
import { computed, useId } from 'vue'
import InfoTip from './InfoTip.vue'

const props = withDefaults(
  defineProps<{
    label: string
    modelValue: string
    placeholder?: string
    /** 常驻的一行说明（校验类、必须一直可见的） */
    hint?: string
    /** 收进「?」里的补充说明：默认不占版面，悬停或聚焦时展开 */
    tip?: string
    error?: string | null
    disabled?: boolean
    maxlength?: number
    type?: 'text' | 'search' | 'number'
  }>(),
  {
    placeholder: undefined,
    hint: undefined,
    tip: undefined,
    error: null,
    disabled: false,
    maxlength: undefined,
    type: 'text',
  },
)

const emit = defineEmits<{
  (event: 'update:modelValue', value: string): void
  (event: 'enter'): void
}>()

const inputId = useId()
const describedBy = computed(() => (props.error ? `${inputId}-error` : props.hint ? `${inputId}-hint` : undefined))

function onInput(event: Event): void {
  emit('update:modelValue', (event.target as HTMLInputElement).value)
}
</script>

<template>
  <div class="field">
    <span class="field__head">
      <label class="field__label" :for="inputId">{{ props.label }}</label>
      <InfoTip v-if="props.tip" :text="props.tip" />
    </span>
    <input
      :id="inputId"
      class="field__input"
      :class="{ 'field__input--error': Boolean(props.error) }"
      :type="props.type"
      :value="props.modelValue"
      :placeholder="props.placeholder"
      :disabled="props.disabled"
      :maxlength="props.maxlength"
      :aria-describedby="describedBy"
      @input="onInput"
      @keydown.enter="emit('enter')"
    />
    <p v-if="props.error" :id="`${inputId}-error`" class="field__message field__message--error">{{ props.error }}</p>
    <p v-else-if="props.hint" :id="`${inputId}-hint`" class="field__message">{{ props.hint }}</p>
  </div>
</template>

<style scoped>
.field {
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
  min-width: 0;
}

.field__head {
  display: inline-flex;
  align-items: center;
  gap: var(--space-2);
}

.field__label {
  font-size: var(--font-sm);
  font-weight: var(--weight-medium);
  color: var(--text-2);
}

.field__input {
  height: var(--control-height);
  padding: 0 var(--space-3);
  border-radius: var(--radius-md);
  border: 1px solid var(--border-1);
  background: var(--surface-1);
  color: var(--text-1);
  transition: border-color var(--transition-fast), box-shadow var(--transition-fast),
    background var(--transition-fast);
  min-width: 0;
  width: 100%;
  outline: none;
}

.field__input::placeholder {
  color: var(--text-3);
}

.field__input:hover:not(:disabled) {
  border-color: var(--border-2);
}

.field__input:focus {
  border-color: var(--accent);
  box-shadow: 0 0 0 3px var(--accent-ring);
}

.field__input--error {
  border-color: var(--error);
}

.field__input--error:focus {
  box-shadow: 0 0 0 3px var(--error-soft);
}

.field__input:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.field__message {
  font-size: var(--font-sm);
  color: var(--text-3);
}

.field__message--error {
  color: var(--error);
}
</style>
