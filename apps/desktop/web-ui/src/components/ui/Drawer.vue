<script setup lang="ts">
/**
 * 右侧抽屉：与 Modal 同一套视觉语言，但从右侧滑入，用于"看详情"这类
 * 需要保留列表上下文的场景（弹窗会把列表遮住，读完就找不回刚才那一条）。
 *
 * 键盘与遮罩行为与 Modal 保持一致：ESC 关闭、点遮罩关闭、打开后焦点进入面板。
 */
import { onBeforeUnmount, onMounted, ref, watch } from 'vue'

const props = withDefaults(
  defineProps<{
    open: boolean
    title: string
    description?: string
    width?: number
  }>(),
  { description: undefined, width: 460 },
)

const emit = defineEmits<{
  (event: 'close'): void
}>()

const panel = ref<HTMLElement | null>(null)

function onKeydown(event: KeyboardEvent): void {
  if (event.key === 'Escape' && props.open) {
    event.stopPropagation()
    emit('close')
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
    <div v-if="props.open" class="scrim" @click.self="emit('close')">
      <aside
        ref="panel"
        class="drawer"
        role="dialog"
        aria-modal="true"
        :aria-label="props.title"
        :style="{ width: `${props.width}px` }"
      >
        <header class="drawer__header">
          <div class="drawer__heading">
            <h2 class="drawer__title">{{ props.title }}</h2>
            <p v-if="props.description" class="drawer__description">{{ props.description }}</p>
          </div>
          <button type="button" class="drawer__close" aria-label="关闭" @click="emit('close')">
            <svg viewBox="0 0 16 16" width="14" height="14" aria-hidden="true">
              <path d="M4 4l8 8M12 4l-8 8" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" />
            </svg>
          </button>
        </header>
        <div class="drawer__body">
          <slot />
        </div>
        <footer v-if="$slots.footer" class="drawer__footer">
          <slot name="footer" />
        </footer>
      </aside>
    </div>
  </Teleport>
</template>

<style scoped>
.scrim {
  position: fixed;
  inset: 0;
  z-index: var(--z-modal);
  background: var(--overlay);
  backdrop-filter: blur(2px);
  display: flex;
  justify-content: flex-end;
  animation: scrim-in var(--transition-base);
}

@keyframes scrim-in {
  from {
    opacity: 0;
  }
}

.drawer {
  height: 100%;
  max-width: calc(100vw - 48px);
  display: flex;
  flex-direction: column;
  background: var(--surface-1);
  border-left: 1px solid var(--border-1);
  box-shadow: var(--shadow-modal);
  animation: drawer-in var(--transition-base);
}

@keyframes drawer-in {
  from {
    transform: translateX(24px);
    opacity: 0.6;
  }
}

.drawer__header {
  flex: none;
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: var(--space-4);
  padding: var(--space-5) var(--space-5) var(--space-4);
  border-bottom: 1px solid var(--border-1);
}

.drawer__heading {
  min-width: 0;
}

.drawer__title {
  font-size: var(--font-lg);
  font-weight: var(--weight-semibold);
  letter-spacing: -0.01em;
  color: var(--text-1);
}

.drawer__description {
  margin-top: 2px;
  font-size: var(--font-sm);
  color: var(--text-3);
}

.drawer__close {
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

.drawer__close:hover {
  background: var(--surface-3);
  color: var(--text-1);
}

.drawer__body {
  flex: 1 1 auto;
  overflow-y: auto;
  padding: var(--space-5);
  display: flex;
  flex-direction: column;
  gap: var(--space-5);
}

.drawer__footer {
  flex: none;
  display: flex;
  align-items: center;
  justify-content: flex-end;
  gap: var(--space-2);
  padding: var(--space-4) var(--space-5);
  border-top: 1px solid var(--border-1);
  background: var(--surface-2);
}
</style>
