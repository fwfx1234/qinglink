<script setup lang="ts">
import Modal from './Modal.vue'
import Button from './Button.vue'

const props = defineProps<{
  open: boolean
  title: string
  message: string
  detail?: string | null
  confirmText: string
  cancelText: string
  tone: 'default' | 'danger'
  busy?: boolean
}>()

const emit = defineEmits<{
  (event: 'confirm'): void
  (event: 'cancel'): void
}>()
</script>

<template>
  <Modal
    :open="props.open"
    :title="props.title"
    :width="420"
    :close-on-backdrop="!props.busy"
    @close="emit('cancel')"
  >
    <p class="confirm__message">{{ props.message }}</p>
    <p v-if="props.detail" class="confirm__detail">{{ props.detail }}</p>
    <template #footer>
      <Button variant="ghost" :disabled="props.busy" @click="emit('cancel')">{{ props.cancelText }}</Button>
      <Button
        :variant="props.tone === 'danger' ? 'danger' : 'primary'"
        :loading="props.busy"
        data-autofocus
        @click="emit('confirm')"
      >
        {{ props.confirmText }}
      </Button>
    </template>
  </Modal>
</template>

<style scoped>
.confirm__message {
  font-size: var(--font-md);
  color: var(--text-1);
}

/* detail 里带手工换行（例如 id 与说明分两行），所以保留换行符 */
.confirm__detail {
  margin-top: var(--space-2);
  padding: var(--space-3);
  border-radius: var(--radius-md);
  background: var(--surface-2);
  border: 1px solid var(--border-1);
  font-size: var(--font-sm);
  color: var(--text-2);
  white-space: pre-line;
  overflow-wrap: anywhere;
}
</style>
