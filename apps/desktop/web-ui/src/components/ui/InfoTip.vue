<script setup lang="ts">
/**
 * 折叠式说明：一个「?」标记，悬停或键盘聚焦时浮出完整文案。
 *
 * 长段提示直接铺在界面里会把操作区挤散，也会让每次都要读的界面反复出现同一段
 * 说明。这里统一收进 tooltip：默认只占一个图标位，需要时再展开。
 *
 * 用 CSS 定位而非 JS 计算，因此不会在滚动/缩放时错位；`aria-describedby` 保证
 * 读屏软件也能拿到同一段文字，键盘用户聚焦即可看见。
 */
import { computed, useId } from 'vue'

const props = withDefaults(
  defineProps<{
    /** 展开后显示的说明文字 */
    text: string
    /** 提示浮层相对标记的位置 */
    placement?: 'top' | 'bottom'
    /** 标记旁的可见短标签（默认只显示「?」） */
    label?: string
  }>(),
  { placement: 'top', label: undefined },
)

const tipId = useId()
const placementClass = computed(() => `info-tip--${props.placement}`)
</script>

<template>
  <span class="info-tip" :class="placementClass">
    <button
      type="button"
      class="info-tip__mark"
      :aria-describedby="tipId"
      :aria-label="props.label ? undefined : '查看说明'"
    >
      <svg viewBox="0 0 16 16" width="12" height="12" aria-hidden="true">
        <circle cx="8" cy="8" r="6.2" fill="none" stroke="currentColor" stroke-width="1.3" />
        <path
          d="M6.4 6.2a1.7 1.7 0 1 1 2.3 1.6c-.5.2-.7.6-.7 1.1v.3"
          fill="none"
          stroke="currentColor"
          stroke-width="1.3"
          stroke-linecap="round"
        />
        <circle cx="8" cy="11.4" r="0.85" fill="currentColor" />
      </svg>
      <span v-if="props.label" class="info-tip__label">{{ props.label }}</span>
    </button>
    <span :id="tipId" class="info-tip__bubble" role="tooltip">{{ props.text }}</span>
  </span>
</template>

<style scoped>
.info-tip {
  position: relative;
  display: inline-flex;
  align-items: center;
  vertical-align: middle;
}

.info-tip__mark {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  padding: 0;
  border: none;
  background: none;
  color: var(--text-3);
  cursor: help;
  transition: color var(--transition-fast);
}

.info-tip__mark:hover,
.info-tip__mark:focus-visible {
  color: var(--accent);
}

.info-tip__label {
  font-size: var(--font-sm);
}

/* 浮层：默认不可见且不接收事件，悬停 / 聚焦时淡入 */
.info-tip__bubble {
  position: absolute;
  left: 0;
  z-index: 30;
  width: max-content;
  max-width: 280px;
  padding: 8px 10px;
  border-radius: var(--radius-md);
  border: 1px solid var(--border-1);
  background: var(--surface-1);
  box-shadow: var(--shadow-pop);
  color: var(--text-2);
  font-size: var(--font-sm);
  font-weight: var(--weight-normal);
  line-height: 1.6;
  white-space: normal;
  text-align: left;
  opacity: 0;
  visibility: hidden;
  transform: translateY(2px);
  transition: opacity var(--transition-fast), transform var(--transition-fast),
    visibility var(--transition-fast);
}

.info-tip--top .info-tip__bubble {
  bottom: calc(100% + 6px);
}

.info-tip--bottom .info-tip__bubble {
  top: calc(100% + 6px);
  transform: translateY(-2px);
}

.info-tip:hover .info-tip__bubble,
.info-tip:focus-within .info-tip__bubble {
  opacity: 1;
  visibility: visible;
  transform: translateY(0);
}
</style>
