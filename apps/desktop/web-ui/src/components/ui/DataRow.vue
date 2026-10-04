<script setup lang="ts">
/**
 * 实体列表行的外壳（应用条目、已配对设备共用）。
 * 行为差异（图标、标签、按钮）通过插槽表达，行高与分隔线的样式只有一份。
 *
 * `clickable` 让整行可以打开详情；行内的选择框与动作区会阻止冒泡，
 * 否则点"启动"或勾选也会顺带把详情面板弹出来。
 */
withDefaults(
  defineProps<{
    muted?: boolean
    clickable?: boolean
    selected?: boolean
  }>(),
  { muted: false, clickable: false, selected: false },
)

defineEmits<{
  (event: 'activate'): void
}>()
</script>

<template>
  <li
    class="data-row"
    :class="{
      'data-row--muted': muted,
      'data-row--clickable': clickable,
      'data-row--selected': selected,
    }"
    :tabindex="clickable ? 0 : undefined"
    @click="clickable && $emit('activate')"
    @keydown.enter="clickable && $emit('activate')"
  >
    <span v-if="$slots.leading" class="data-row__leading" @click.stop>
      <slot name="leading" />
    </span>

    <div class="data-row__main">
      <slot />
    </div>

    <div v-if="$slots.actions" class="data-row__actions" @click.stop>
      <slot name="actions" />
    </div>
  </li>
</template>

<style scoped>
.data-row {
  position: relative;
  display: flex;
  align-items: center;
  gap: var(--space-3);
  padding: var(--space-3) var(--space-5);
  border-bottom: 1px solid var(--border-1);
  transition: background var(--transition-fast);
}

.data-row:last-child {
  border-bottom: none;
}

.data-row:hover {
  background: var(--surface-2);
}

.data-row--clickable {
  cursor: pointer;
}

.data-row--clickable:focus-visible {
  outline: 2px solid var(--accent);
  outline-offset: -2px;
}

/* 选中行：左侧强调条 + 浅色底，比只给复选框打勾更容易扫到 */
.data-row--selected {
  background: var(--accent-soft);
}

.data-row--selected::before {
  content: '';
  position: absolute;
  left: 0;
  top: 0;
  bottom: 0;
  width: 3px;
  background: var(--accent);
}

.data-row--muted {
  background: var(--warning-soft-faint);
}

.data-row--muted:hover {
  background: var(--warning-soft-strong);
}

.data-row__leading {
  flex: none;
  display: inline-flex;
  align-items: center;
  gap: var(--space-2);
}

.data-row__main {
  flex: 1 1 auto;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 3px;
}

.data-row__actions {
  display: flex;
  align-items: center;
  gap: var(--space-1);
  flex: none;
}

@media (max-width: 1180px) {
  .data-row {
    flex-wrap: wrap;
  }

  .data-row__actions {
    width: 100%;
    justify-content: flex-end;
  }
}
</style>
