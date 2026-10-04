<script setup lang="ts">
withDefaults(
  defineProps<{
    title?: string
    subtitle?: string
    /** 卡片内嵌区块的视觉层级 */
    tone?: 'default' | 'raised'
    padding?: 'none' | 'md' | 'lg'
  }>(),
  { title: undefined, subtitle: undefined, tone: 'default', padding: 'md' },
)
</script>

<template>
  <section class="card" :class="[`card--${tone}`, `card--pad-${padding}`]">
    <header v-if="title || $slots.header || $slots.actions" class="card__header">
      <div class="card__heading">
        <slot name="header">
          <h2 class="card__title">{{ title }}</h2>
          <p v-if="subtitle" class="card__subtitle">{{ subtitle }}</p>
        </slot>
      </div>
      <div v-if="$slots.actions" class="card__actions">
        <slot name="actions" />
      </div>
    </header>
    <div class="card__body">
      <slot />
    </div>
    <footer v-if="$slots.footer" class="card__footer">
      <slot name="footer" />
    </footer>
  </section>
</template>

<style scoped>
.card {
  background: var(--surface-1);
  border: 1px solid var(--border-1);
  border-radius: var(--radius-lg);
  box-shadow: var(--shadow-card);
  display: flex;
  flex-direction: column;
  min-width: 0;
}

.card--raised {
  background: var(--surface-2);
  box-shadow: none;
}

/* `padding` 只控制**内容区**：列表卡片需要内容贴边（行自带内边距），但头部与底部
   始终保留一档内边距，否则标题和按钮会贴到卡片边缘。 */
.card__header,
.card__footer {
  padding: var(--space-4) var(--space-5);
}

.card--pad-none .card__body {
  padding: 0;
}

.card--pad-md .card__body {
  padding: var(--space-5);
}

.card--pad-lg .card__header,
.card--pad-lg .card__footer {
  padding: var(--space-4) var(--space-6);
}

.card--pad-lg .card__body {
  padding: var(--space-6);
}

.card__header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--space-4);
  border-bottom: 1px solid var(--border-1);
}

.card__heading {
  min-width: 0;
}

.card__title {
  font-size: var(--font-lg);
  font-weight: var(--weight-semibold);
  letter-spacing: -0.02em;
  color: var(--text-1);
}

.card__subtitle {
  margin-top: 2px;
  font-size: var(--font-sm);
  color: var(--text-3);
}

.card__actions {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  flex-wrap: wrap;
  justify-content: flex-end;
}

.card__body {
  flex: 1 1 auto;
  min-width: 0;
}

.card__footer {
  border-top: 1px solid var(--border-1);
  color: var(--text-2);
  font-size: var(--font-sm);
  margin-top: auto;
}
</style>
