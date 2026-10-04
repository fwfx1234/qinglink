<script setup lang="ts">
import { computed } from 'vue'
import { useThemeStore, type ThemeMode } from '@/stores/theme'

const theme = useThemeStore()

/** 三态循环：浅色（默认）→ 深色 → 跟随系统 → 浅色。 */
const ORDER: ThemeMode[] = ['light', 'dark', 'system']

const LABEL: Record<ThemeMode, string> = {
  system: '跟随系统',
  light: '浅色',
  dark: '深色',
}

const nextMode = computed<ThemeMode>(() => {
  const index = ORDER.indexOf(theme.mode)
  return ORDER[(index + 1) % ORDER.length] ?? 'system'
})

const label = computed(() => LABEL[theme.mode])
const nextLabel = computed(() => LABEL[nextMode.value])
const hint = computed(
  () => `主题：${label.value}（当前显示为${theme.isDark ? '深色' : '浅色'}）· 点击切换到${nextLabel.value}`,
)

function cycle(): void {
  theme.setMode(nextMode.value)
}
</script>

<template>
  <button
    type="button"
    class="theme-toggle"
    :title="hint"
    :aria-label="hint"
    :data-mode="theme.mode"
    @click="cycle"
  >
    <svg class="theme-toggle__icon" viewBox="0 0 24 24" aria-hidden="true">
      <!-- 深色：月亮 -->
      <path
        v-if="theme.isDark"
        d="M20 14.5A8.5 8.5 0 0 1 9.5 4a8.5 8.5 0 1 0 10.5 10.5Z"
        fill="currentColor"
      />
      <!-- 浅色：太阳 -->
      <g v-else stroke="currentColor" stroke-width="1.8" stroke-linecap="round">
        <circle cx="12" cy="12" r="4.2" fill="currentColor" stroke="none" />
        <path d="M12 2.6v2.2M12 19.2v2.2M2.6 12h2.2M19.2 12h2.2M5.4 5.4l1.6 1.6M17 17l1.6 1.6M18.6 5.4 17 7M7 17l-1.6 1.6" />
      </g>
    </svg>
    <span class="theme-toggle__text">{{ label }}</span>
  </button>
</template>

<style scoped>
.theme-toggle {
  display: inline-flex;
  align-items: center;
  gap: var(--space-2);
  height: var(--control-height);
  padding: 0 var(--space-3);
  border: 1px solid var(--border-2);
  border-radius: var(--radius-md);
  background: var(--surface-1);
  color: var(--text-2);
  font-size: var(--font-sm);
  font-weight: var(--weight-medium);
  cursor: pointer;
  box-shadow: var(--shadow-xs);
  transition: background var(--transition-fast), border-color var(--transition-fast),
    color var(--transition-fast), box-shadow var(--transition-fast);
}

.theme-toggle:hover {
  background: var(--surface-3);
  border-color: var(--text-3);
  color: var(--text-1);
}

.theme-toggle:focus-visible {
  outline: none;
  box-shadow: 0 0 0 3px var(--accent-ring);
}

.theme-toggle__icon {
  width: 14px;
  height: 14px;
  flex: none;
}

@media (max-width: 900px) {
  .theme-toggle__text {
    display: none;
  }

  .theme-toggle {
    padding: 0 var(--space-2);
  }
}
</style>
