<script setup lang="ts">
/**
 * 提示块：一块柔和的着色面板，而不是"卡片里再套一段文字"。
 * tone 只改左侧色条与衬底色，正文仍是标准前景色（对比度由令牌保证）。
 */
withDefaults(
  defineProps<{
    title?: string
    tone?: 'default' | 'accent' | 'warning' | 'error'
    compact?: boolean
  }>(),
  { title: undefined, tone: 'default', compact: false },
)
</script>

<template>
  <div class="callout" :class="[`callout--${tone}`, { 'callout--compact': compact }]">
    <p v-if="title" class="callout__title">{{ title }}</p>
    <div class="callout__body">
      <slot />
    </div>
  </div>
</template>

<style scoped>
.callout {
  position: relative;
  padding: var(--space-3) var(--space-4) var(--space-3) var(--space-5);
  border: 1px solid var(--border-1);
  border-radius: var(--radius-md);
  background: var(--surface-1);
  color: var(--text-2);
  font-size: var(--font-md);
  line-height: 1.6;
  overflow: hidden;
}

/* 左侧色条：语义在不改变文字颜色的前提下可见 */
.callout::before {
  content: '';
  position: absolute;
  left: 0;
  top: 0;
  bottom: 0;
  width: 3px;
  background: var(--border-2);
}

.callout--compact {
  padding: var(--space-2) var(--space-3) var(--space-2) var(--space-4);
  font-size: var(--font-sm);
}

.callout--accent {
  background: var(--accent-soft);
  border-color: var(--accent-border);
}

.callout--accent::before {
  background: var(--accent-bright);
}

.callout--warning {
  background: var(--warning-soft);
  border-color: var(--warning-border);
}

.callout--warning::before {
  background: var(--warning);
}

.callout--error {
  background: var(--error-soft);
  border-color: var(--error-border);
}

.callout--error::before {
  background: var(--error);
}

.callout__title {
  margin-bottom: 2px;
  font-size: var(--font-sm);
  font-weight: var(--weight-semibold);
  color: var(--text-1);
}

.callout__body :deep(strong) {
  color: var(--text-1);
  font-weight: var(--weight-semibold);
}
</style>
