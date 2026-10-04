<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import type { AppItem } from '@/api/types'
import { APP_NAME_MAX_LENGTH } from '@/utils/constants'
import Modal from '@/components/ui/Modal.vue'
import Button from '@/components/ui/Button.vue'
import TextField from '@/components/ui/TextField.vue'

const props = defineProps<{
  app: AppItem | null
  busy: boolean
}>()

const emit = defineEmits<{
  (event: 'submit', payload: { id: string; name: string }): void
  (event: 'close'): void
}>()

const draft = ref('')
const validation = ref<string | null>(null)

watch(
  () => props.app,
  (app) => {
    draft.value = app?.name ?? ''
    validation.value = null
  },
  { immediate: true },
)

const originalLabel = computed(() => props.app?.original_name ?? '')
const trimmed = computed(() => draft.value.trim())
const willRestoreOriginal = computed(() => trimmed.value === '')
const unchanged = computed(() => props.app !== null && trimmed.value === props.app.name)

function validate(): string | null {
  if (trimmed.value === '') return null
  // 后端按 Unicode 标量计数（chars().count()），这里同样按码点算，
  // 避免代理对被当作两个字符而提前拒绝或放行。
  if ([...trimmed.value].length > APP_NAME_MAX_LENGTH) {
    return `名称长度不能超过 ${APP_NAME_MAX_LENGTH} 个字符`
  }
  // 契约 §4.17：不得包含控制字符
  for (const character of trimmed.value) {
    const code = character.codePointAt(0) ?? 0
    if (code < 0x20 || code === 0x7f) return '名称不能包含控制字符'
  }
  return null
}

function submit(): void {
  if (!props.app) return
  const error = validate()
  validation.value = error
  if (error) return
  emit('submit', { id: props.app.id, name: trimmed.value })
}
</script>

<template>
  <Modal
    :open="props.app !== null"
    title="重命名条目"
    description="重命名写入服务端目录，移动端下一次拉取即生效"
    :close-on-backdrop="!props.busy"
    @close="emit('close')"
  >
    <div class="rename__path">
      <span class="rename__path-label">路径</span>
      <span class="rename__path-value">{{ props.app?.exe_path || props.app?.id }}</span>
    </div>

    <TextField
      v-model="draft"
      label="显示名称"
      :maxlength="APP_NAME_MAX_LENGTH"
      :error="validation"
      :disabled="props.busy"
      placeholder="留空表示恢复原始名"
      :hint="`原名：${originalLabel || '—'}；留空提交等于恢复原始名`"
      @enter="submit"
    />

    <p v-if="willRestoreOriginal" class="rename__note">将清空自定义名称，恢复为扫描得到的原始名。</p>
    <p v-else-if="unchanged" class="rename__note">名称没有变化，提交不会产生修改。</p>

    <template #footer>
      <Button variant="ghost" :disabled="props.busy" @click="emit('close')">取消</Button>
      <Button variant="primary" :loading="props.busy" :disabled="unchanged" @click="submit">保存名称</Button>
    </template>
  </Modal>
</template>

<style scoped>
.rename__path {
  display: flex;
  flex-direction: column;
  gap: var(--space-1);
  padding: var(--space-3);
  border-radius: var(--radius-md);
  background: var(--surface-2);
  border: 1px solid var(--border-1);
}

.rename__path-label {
  font-size: var(--font-xs);
  color: var(--text-3);
  text-transform: uppercase;
  letter-spacing: 0.06em;
}

.rename__path-value {
  font-family: var(--font-mono);
  font-size: var(--font-sm);
  color: var(--text-2);
  word-break: break-all;
}

.rename__note {
  font-size: var(--font-sm);
  color: var(--text-3);
}
</style>
