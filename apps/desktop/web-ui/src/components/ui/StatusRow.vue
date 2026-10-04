<script setup lang="ts">
/** 标签 + 值的状态行：服务状态、体检项、进程信息共用同一种排版。 */
withDefaults(
  defineProps<{
    label: string
    value?: string | number | null
    mono?: boolean
  }>(),
  { value: undefined, mono: false },
)
</script>

<template>
  <div class="status-row">
    <span class="status-row__label">{{ label }}</span>
    <span class="status-row__value" :class="{ 'status-row__value--mono': mono }">
      <slot>{{ value ?? '—' }}</slot>
    </span>
  </div>
</template>

<style scoped>
.status-row {
  display: flex;
  align-items: baseline;
  justify-content: space-between;
  gap: var(--space-4);
  padding: var(--space-2) 0;
  border-bottom: 1px solid var(--border-1);
  min-width: 0;
}

.status-row:last-child {
  border-bottom: none;
}

.status-row__label {
  font-size: var(--font-sm);
  color: var(--text-3);
  flex: none;
}

.status-row__value {
  display: inline-flex;
  align-items: center;
  gap: var(--space-2);
  font-size: var(--font-sm);
  font-weight: var(--weight-medium);
  color: var(--text-1);
  text-align: right;
  min-width: 0;
  overflow-wrap: anywhere;
  font-variant-numeric: tabular-nums;
}

.status-row__value--mono {
  font-family: var(--font-mono);
  font-weight: var(--weight-normal);
}
</style>
