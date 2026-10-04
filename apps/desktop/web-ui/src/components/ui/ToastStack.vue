<script setup lang="ts">
import { useToast } from '@/composables/useToast'

const { items, dismiss } = useToast()
</script>

<template>
  <Teleport to="body">
    <div v-if="items.length > 0" class="toast-stack" role="status" aria-live="polite">
      <div v-for="item in items" :key="item.id" class="toast" :class="`toast--${item.level}`">
        <span class="toast__marker" aria-hidden="true" />
        <span class="toast__message">{{ item.message }}</span>
        <button type="button" class="toast__close" aria-label="关闭提示" @click="dismiss(item.id)">
          <svg viewBox="0 0 16 16" width="12" height="12" aria-hidden="true">
            <path d="M4 4l8 8M12 4l-8 8" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" />
          </svg>
        </button>
      </div>
    </div>
  </Teleport>
</template>

<style scoped>
.toast-stack {
  position: fixed;
  right: var(--space-5);
  bottom: var(--space-5);
  z-index: var(--z-toast);
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
  width: min(380px, calc(100vw - 32px));
}

.toast {
  display: flex;
  align-items: flex-start;
  gap: var(--space-3);
  padding: var(--space-3) var(--space-4);
  border-radius: var(--radius-md);
  border: 1px solid var(--border-1);
  background: var(--surface-1);
  box-shadow: var(--shadow-pop);
  animation: toast-in var(--transition-base);
}

@keyframes toast-in {
  from {
    opacity: 0;
    transform: translateY(6px);
  }
}

.toast__marker {
  width: 3px;
  align-self: stretch;
  border-radius: var(--radius-pill);
  background: var(--text-3);
  flex: none;
}

.toast--info .toast__marker {
  background: var(--accent);
}

.toast--warn .toast__marker {
  background: var(--warning);
}

.toast--error .toast__marker {
  background: var(--error);
}

.toast__message {
  flex: 1 1 auto;
  font-size: var(--font-sm);
  color: var(--text-1);
  word-break: break-word;
}

.toast__close {
  flex: none;
  width: 20px;
  height: 20px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  border: none;
  border-radius: var(--radius-sm);
  background: transparent;
  color: var(--text-3);
  cursor: pointer;
  transition: background var(--transition-fast), color var(--transition-fast);
}

.toast__close:hover {
  background: var(--surface-3);
  color: var(--text-1);
}
</style>
