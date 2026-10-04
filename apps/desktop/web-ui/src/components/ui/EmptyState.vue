<script setup lang="ts">
import Button from '@/components/ui/Button.vue'

withDefaults(
  defineProps<{
    title: string
    description?: string
    /** 次要的引导动作文案 */
    actionLabel?: string
    tone?: 'neutral' | 'accent' | 'warning' | 'error'
    compact?: boolean
  }>(),
  { description: undefined, actionLabel: undefined, tone: 'neutral', compact: false },
)

defineEmits<{
  (event: 'action'): void
}>()
</script>

<template>
  <div class="empty" :class="[`empty--${tone}`, { 'empty--compact': compact }]">
    <slot name="icon">
      <svg class="empty__icon" viewBox="0 0 24 24" width="22" height="22" aria-hidden="true">
        <path
          d="M3 7.5A2.5 2.5 0 0 1 5.5 5h4l1.6 2H18.5A2.5 2.5 0 0 1 21 9.5v7A2.5 2.5 0 0 1 18.5 19h-13A2.5 2.5 0 0 1 3 16.5v-9Z"
          fill="none"
          stroke="currentColor"
          stroke-width="1.4"
          stroke-linejoin="round"
        />
      </svg>
    </slot>
    <p class="empty__title">{{ title }}</p>
    <p v-if="description" class="empty__description">{{ description }}</p>
    <div v-if="actionLabel || $slots.action" class="empty__action">
      <slot name="action">
        <Button size="sm" @click="$emit('action')">{{ actionLabel }}</Button>
      </slot>
    </div>
  </div>
</template>

<style scoped>
.empty {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: var(--space-2);
  padding: var(--space-8) var(--space-4);
  text-align: center;
  color: var(--text-2);
}

.empty--compact {
  padding: var(--space-5) var(--space-3);
}

/* 图标坐在一块柔和底色上，比裸图标更有设计感 */
.empty__icon {
  padding: 8px;
  box-sizing: content-box;
  border-radius: var(--radius-pill);
  background: var(--surface-3);
  color: var(--text-3);
  margin-bottom: var(--space-1);
}

.empty--accent .empty__icon {
  background: var(--accent-soft);
  color: var(--accent);
}

.empty--warning .empty__icon {
  background: var(--warning-soft);
  color: var(--warning);
}

.empty--error .empty__icon {
  background: var(--error-soft);
  color: var(--error);
}

.empty__title {
  font-size: var(--font-lg);
  font-weight: var(--weight-semibold);
  color: var(--text-1);
}

.empty__description {
  font-size: var(--font-sm);
  color: var(--text-3);
  max-width: 46ch;
}

.empty__action {
  margin-top: var(--space-3);
}
</style>
