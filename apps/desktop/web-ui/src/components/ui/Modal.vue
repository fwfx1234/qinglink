<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref, watch } from 'vue'

const props = withDefaults(
  defineProps<{
    open: boolean
    title: string
    description?: string
    width?: number
    /** 点击遮罩是否关闭 */
    closeOnBackdrop?: boolean
  }>(),
  { description: undefined, width: 460, closeOnBackdrop: true },
)

const emit = defineEmits<{
  (event: 'close'): void
}>()

const panel = ref<HTMLElement | null>(null)

function close(): void {
  emit('close')
}

function onKeydown(event: KeyboardEvent): void {
  if (event.key === 'Escape' && props.open) {
    event.stopPropagation()
    close()
  }
}

watch(
  () => props.open,
  (open) => {
    if (open) {
      window.addEventListener('keydown', onKeydown)
      queueMicrotask(() => {
        panel.value?.querySelector<HTMLElement>('[data-autofocus]')?.focus()
      })
    } else {
      window.removeEventListener('keydown', onKeydown)
    }
  },
  { immediate: true },
)

onMounted(() => {
  if (props.open) window.addEventListener('keydown', onKeydown)
})

onBeforeUnmount(() => {
  window.removeEventListener('keydown', onKeydown)
})
</script>

<template>
  <Teleport to="body">
    <div v-if="props.open" class="overlay" @click.self="props.closeOnBackdrop && close()">
      <div
        ref="panel"
        class="panel"
        role="dialog"
        aria-modal="true"
        :aria-label="props.title"
        :style="{ width: `${props.width}px` }"
      >
        <header class="panel__header">
          <div>
            <h2 class="panel__title">{{ props.title }}</h2>
            <p v-if="props.description" class="panel__description">{{ props.description }}</p>
          </div>
          <button type="button" class="panel__close" aria-label="关闭" @click="close">
            <svg viewBox="0 0 16 16" width="14" height="14" aria-hidden="true">
              <path d="M4 4l8 8M12 4l-8 8" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" />
            </svg>
          </button>
        </header>
        <div class="panel__body">
          <slot />
        </div>
        <footer v-if="$slots.footer" class="panel__footer">
          <slot name="footer" />
        </footer>
      </div>
    </div>
  </Teleport>
</template>

<style scoped>
.overlay {
  position: fixed;
  inset: 0;
  z-index: var(--z-modal);
  background: var(--overlay);
  backdrop-filter: blur(3px);
  display: flex;
  align-items: flex-start;
  justify-content: center;
  padding: 12vh var(--space-4) var(--space-4);
  overflow-y: auto;
  animation: overlay-in var(--transition-base);
}

@keyframes overlay-in {
  from {
    opacity: 0;
  }
}

.panel {
  max-width: calc(100vw - 32px);
  background: var(--surface-1);
  border: 1px solid var(--border-1);
  border-radius: var(--radius-xl);
  box-shadow: var(--shadow-modal);
  overflow: hidden;
  animation: panel-in var(--transition-base);
}

@keyframes panel-in {
  from {
    opacity: 0;
    transform: translateY(-6px) scale(0.99);
  }
}

.panel__header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: var(--space-4);
  padding: var(--space-4) var(--space-5);
  border-bottom: 1px solid var(--border-1);
}

.panel__title {
  font-size: var(--font-lg);
  font-weight: var(--weight-semibold);
  letter-spacing: -0.01em;
}

.panel__description {
  margin-top: var(--space-1);
  font-size: var(--font-sm);
  color: var(--text-3);
}

.panel__close {
  flex: none;
  width: 30px;
  height: 30px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  border: 1px solid transparent;
  border-radius: var(--radius-md);
  background: transparent;
  color: var(--text-3);
  cursor: pointer;
  transition: background var(--transition-fast), color var(--transition-fast);
}

.panel__close:hover {
  background: var(--surface-3);
  color: var(--text-1);
}

.panel__body {
  padding: var(--space-5);
  display: flex;
  flex-direction: column;
  gap: var(--space-4);
}

.panel__footer {
  display: flex;
  align-items: center;
  justify-content: flex-end;
  gap: var(--space-2);
  padding: var(--space-4) var(--space-5);
  border-top: 1px solid var(--border-1);
  background: var(--surface-2);
}
</style>
