<script setup lang="ts">
/**
 * 行内「⋯」菜单。
 *
 * 列表行的动作太多时（重命名 / 图标 / 隐藏 / 恢复原名 / 删除），把它们收进一个
 * 弹出菜单，行上只留最常用的两三个按钮，视觉密度立刻降下来。
 *
 * 菜单渲染到 body 并用 fixed 定位：列表行有自己的层叠与溢出上下文，就地定位会
 * 被裁掉。窗口滚动或缩放时直接关闭——这比跟随重排更不容易错位。
 */
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue'

export interface RowMenuItem {
  id: string
  label: string
  /** 危险动作：悬停显示红色 */
  danger?: boolean
  disabled?: boolean
  /** 菜单项右侧的补充说明（例如"已设置 2 个"） */
  hint?: string
}

const props = withDefaults(
  defineProps<{
    items: RowMenuItem[]
    label?: string
    disabled?: boolean
    align?: 'start' | 'end'
  }>(),
  { label: '更多操作', disabled: false, align: 'end' },
)

const emit = defineEmits<{
  (event: 'select', id: string): void
}>()

const open = ref(false)
const trigger = ref<HTMLElement | null>(null)
const menu = ref<HTMLElement | null>(null)
const position = ref({ top: 0, left: 0 })

const MENU_WIDTH = 208

function place(): void {
  const rect = trigger.value?.getBoundingClientRect()
  if (!rect) return
  const height = menu.value?.offsetHeight ?? props.items.length * 34 + 12
  // 视口底部放不下时向上翻转，避免菜单被裁掉。
  const below = rect.bottom + 6
  const top = below + height > window.innerHeight - 8 ? Math.max(8, rect.top - height - 6) : below
  const rawLeft = props.align === 'end' ? rect.right - MENU_WIDTH : rect.left
  const left = Math.min(Math.max(8, rawLeft), window.innerWidth - MENU_WIDTH - 8)
  position.value = { top, left }
}

function close(): void {
  open.value = false
}

function toggle(): void {
  if (props.disabled) return
  open.value = !open.value
}

function onSelect(item: RowMenuItem): void {
  if (item.disabled) return
  close()
  emit('select', item.id)
}

function onDocumentPointerDown(event: PointerEvent): void {
  const target = event.target as Node
  if (menu.value?.contains(target) || trigger.value?.contains(target)) return
  close()
}

function onKeydown(event: KeyboardEvent): void {
  if (event.key === 'Escape') close()
}

watch(open, (value) => {
  if (value) {
    // 先挂载菜单才能量到高度，所以放到下一帧再定位。
    queueMicrotask(place)
    document.addEventListener('pointerdown', onDocumentPointerDown, true)
    document.addEventListener('keydown', onKeydown)
    window.addEventListener('resize', close)
    window.addEventListener('scroll', close, true)
  } else {
    document.removeEventListener('pointerdown', onDocumentPointerDown, true)
    document.removeEventListener('keydown', onKeydown)
    window.removeEventListener('resize', close)
    window.removeEventListener('scroll', close, true)
  }
})

onMounted(() => {
  if (open.value) place()
})

onBeforeUnmount(() => {
  document.removeEventListener('pointerdown', onDocumentPointerDown, true)
  document.removeEventListener('keydown', onKeydown)
  window.removeEventListener('resize', close)
  window.removeEventListener('scroll', close, true)
})

const triggerClass = computed(() => ['row-menu__trigger', { 'row-menu__trigger--open': open.value }])
</script>

<template>
  <span ref="trigger" class="row-menu">
    <button
      type="button"
      :class="triggerClass"
      :disabled="props.disabled"
      :aria-label="props.label"
      :aria-expanded="open"
      title="更多操作"
      @click.stop="toggle"
    >
      <svg viewBox="0 0 16 16" width="15" height="15" aria-hidden="true">
        <circle cx="3.2" cy="8" r="1.35" fill="currentColor" />
        <circle cx="8" cy="8" r="1.35" fill="currentColor" />
        <circle cx="12.8" cy="8" r="1.35" fill="currentColor" />
      </svg>
    </button>
    <Teleport to="body">
      <div
        v-if="open"
        ref="menu"
        class="row-menu__popover"
        role="menu"
        :style="{ top: `${position.top}px`, left: `${position.left}px`, width: `${MENU_WIDTH}px` }"
      >
        <button
          v-for="item in props.items"
          :key="item.id"
          type="button"
          class="row-menu__item"
          :class="{ 'row-menu__item--danger': item.danger }"
          role="menuitem"
          :disabled="item.disabled"
          @click.stop="onSelect(item)"
        >
          <span class="row-menu__label">{{ item.label }}</span>
          <span v-if="item.hint" class="row-menu__hint">{{ item.hint }}</span>
        </button>
      </div>
    </Teleport>
  </span>
</template>

<style scoped>
.row-menu {
  display: inline-flex;
}

.row-menu__trigger {
  width: 30px;
  height: 30px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  border: 1px solid transparent;
  border-radius: var(--radius-md);
  background: transparent;
  color: var(--text-2);
  cursor: pointer;
  transition: background var(--transition-fast), color var(--transition-fast);
}

.row-menu__trigger:hover:not(:disabled),
.row-menu__trigger--open {
  background: var(--surface-3);
  color: var(--text-1);
}

.row-menu__trigger:disabled {
  cursor: not-allowed;
  opacity: 0.45;
}

.row-menu__popover {
  position: fixed;
  z-index: var(--z-modal);
  padding: var(--space-1);
  background: var(--surface-1);
  border: 1px solid var(--border-1);
  border-radius: var(--radius-lg);
  box-shadow: var(--shadow-pop);
  display: flex;
  flex-direction: column;
  gap: 1px;
  animation: menu-in var(--transition-fast);
}

@keyframes menu-in {
  from {
    opacity: 0;
    transform: translateY(-4px);
  }
}

.row-menu__item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--space-3);
  width: 100%;
  padding: 7px var(--space-3);
  border: none;
  border-radius: var(--radius-md);
  background: transparent;
  color: var(--text-1);
  font-size: var(--font-sm);
  text-align: left;
  cursor: pointer;
  transition: background var(--transition-fast), color var(--transition-fast);
}

.row-menu__item:hover:not(:disabled) {
  background: var(--surface-3);
}

.row-menu__item--danger {
  color: var(--error);
}

.row-menu__item--danger:hover:not(:disabled) {
  background: var(--error-soft);
}

.row-menu__item:disabled {
  cursor: not-allowed;
  opacity: 0.45;
}

.row-menu__hint {
  font-size: var(--font-xs);
  color: var(--text-3);
}
</style>
