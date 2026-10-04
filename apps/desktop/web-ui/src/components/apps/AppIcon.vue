<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { appsApi } from '@/api/endpoints'

const props = withDefaults(
  defineProps<{
    id: string
    /**
     * 是否已缓存图标。图标接口本身会按需解析，所以这里只是提示：
     * 即使为 false 也照样渲染 <img>，失败时再回退到占位字母。
     */
    hasIcon: boolean
    size?: number
    /** icon_updated_at；换图标后变化，用于绕开图标 URL 的长缓存 */
    version?: number | null
    /** 显示名：没取到图标时用它的首字母做占位，比 id 尾字符可读得多 */
    name?: string | null
  }>(),
  { size: 32, version: null, name: null },
)

const failed = ref(false)

// 契约 §4.16 已给出可直接用于 <img> 的路径（内部按 /api/admin/v1/apps/icon 构造）
const source = computed(() => appsApi.iconUrl(props.id, props.version))

/**
 * 占位字母：优先取显示名。中文取前两个字，拉丁文取单词首字母（最多两个），
 * 都没有才退回 id 的末两个字符。
 */
const initials = computed(() => {
  const name = (props.name ?? '').trim()
  if (name) {
    const words = name.split(/[\s\-_.]+/).filter(Boolean)
    if (words.length >= 2 && /^[\x20-\x7e]+$/.test(name)) {
      return words
        .slice(0, 2)
        .map((word) => word[0] ?? '')
        .join('')
        .toUpperCase()
    }
    return name.slice(0, 2).toUpperCase()
  }
  return props.id.replace(/[\\/]+$/, '').slice(-2).toUpperCase()
})

watch(
  () => [props.id, props.version],
  () => {
    failed.value = false
  },
)
</script>

<template>
  <span class="icon" :style="{ width: `${size}px`, height: `${size}px` }">
    <img
      v-if="!failed"
      class="icon__image"
      :src="source"
      :alt="''"
      loading="lazy"
      decoding="async"
      @error="failed = true"
    />
    <span v-else class="icon__placeholder" aria-hidden="true">{{ initials }}</span>
  </span>
</template>

<style scoped>
.icon {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  flex: none;
  border-radius: var(--radius-sm);
  border: 1px solid var(--border-1);
  background: var(--surface-3);
  overflow: hidden;
}

.icon__image {
  width: 100%;
  height: 100%;
  object-fit: contain;
}

.icon__placeholder {
  font-family: var(--font-mono);
  font-size: var(--font-xs);
  color: var(--text-3);
  text-transform: uppercase;
}
</style>
