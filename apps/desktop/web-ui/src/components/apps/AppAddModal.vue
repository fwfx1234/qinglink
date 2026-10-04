<script setup lang="ts">
/**
 * 新增自定义条目（§4.31）。
 *
 * 浏览器拿不到本机文件路径，所以这里提供「浏览…」：由桌面端弹出原生选择窗口并
 * 回传绝对路径（§4.35）。手填仍然保留，粘贴路径比在树里翻找更快。真正校验在
 * 服务端完成（路径必须是存在的文件，重复路径返回 CONFLICT）。
 */
import { computed, ref, watch } from 'vue'
import type { AppAddRequest, CatalogKind } from '@/api/types'
import { dialogApi } from '@/api/endpoints'
import { APP_NAME_MAX_LENGTH } from '@/utils/constants'
import { toAppError } from '@/utils/errors'
import Modal from '@/components/ui/Modal.vue'
import Button from '@/components/ui/Button.vue'
import TextField from '@/components/ui/TextField.vue'
import SelectField from '@/components/ui/SelectField.vue'
import Callout from '@/components/ui/Callout.vue'

const props = defineProps<{
  open: boolean
  busy: boolean
  /** 预设类型：跟随目录页当前分类 */
  defaultKind: CatalogKind
  error: string | null
}>()

const emit = defineEmits<{
  (event: 'submit', payload: AppAddRequest): void
  (event: 'close'): void
}>()

const path = ref('')
const name = ref('')
const kind = ref<CatalogKind>(props.defaultKind)
const argsText = ref('')
const picking = ref(false)
const pickError = ref<string | null>(null)

const kindOptions = [
  { value: 'app', label: '应用' },
  { value: 'game', label: '游戏' },
]

watch(
  () => props.open,
  (open) => {
    if (!open) return
    path.value = ''
    name.value = ''
    argsText.value = ''
    kind.value = props.defaultKind
    pickError.value = null
  },
)

const pathError = computed(() =>
  path.value.trim().length === 0 ? '请选择或填写可执行文件路径' : null,
)

/** 从文件名猜一个显示名，省掉一次输入；用户可以再改。 */
function guessName(value: string): string {
  const file = value.split(/[\\/]/).pop() ?? ''
  return file.replace(/\.(exe|lnk)$/i, '')
}

async function browse(): Promise<void> {
  if (picking.value) return
  picking.value = true
  pickError.value = null
  try {
    const result = await dialogApi.pickFile('executable')
    if (!result.path) return
    path.value = result.path
    if (name.value.trim().length === 0) name.value = guessName(result.path)
  } catch (caught) {
    pickError.value = toAppError(caught).message
  } finally {
    picking.value = false
  }
}

/** 按空白切分启动参数，双引号内的空白不切分。 */
function parseArgs(text: string): string[] {
  const args: string[] = []
  let current = ''
  let quoted = false
  for (const character of text.trim()) {
    if (character === '"') {
      quoted = !quoted
      continue
    }
    if (!quoted && /\s/.test(character)) {
      if (current !== '') {
        args.push(current)
        current = ''
      }
      continue
    }
    current += character
  }
  if (current !== '') args.push(current)
  return args.slice(0, 64)
}

function submit(): void {
  if (pathError.value) return
  emit('submit', {
    path: path.value.trim(),
    name: name.value.trim(),
    kind: kind.value,
    args: parseArgs(argsText.value),
  })
}
</script>

<template>
  <Modal
    :open="props.open"
    title="新增目录条目"
    description="把本机上的一个程序登记到目录，手机端与目录页会立即看到它。"
    :width="520"
    @close="emit('close')"
  >
    <div class="picker">
      <TextField
        v-model="path"
        class="picker__field"
        label="可执行文件路径"
        placeholder="C:\Program Files\Example\example.exe"
        :tip="'必须是本机已存在的文件；可以从资源管理器拖不进来，所以用「浏览…」选或直接粘贴。'"
        :error="pathError"
        data-autofocus
      />
      <Button size="md" variant="secondary" :loading="picking" @click="browse">浏览…</Button>
    </div>

    <TextField
      v-model="name"
      label="显示名（可留空）"
      placeholder="留空则使用文件名"
      :maxlength="APP_NAME_MAX_LENGTH"
      :tip="'名称会同时生效于本机目录与所有已配对客户端。'"
    />

    <SelectField
      :model-value="kind"
      label="类型"
      :options="kindOptions"
      @update:model-value="kind = $event as CatalogKind"
    />

    <TextField
      v-model="argsText"
      label="启动参数（可留空）"
      placeholder="--fullscreen &quot;my profile&quot;"
      :tip="'按空白切分；参数本身含空格时用双引号包起来。'"
    />

    <Callout v-if="props.error || pickError" tone="error" compact>
      {{ props.error ?? pickError }}
    </Callout>

    <template #footer>
      <Button size="sm" variant="ghost" :disabled="props.busy" @click="emit('close')">取消</Button>
      <Button size="sm" variant="primary" :loading="props.busy" @click="submit">添加</Button>
    </template>
  </Modal>
</template>

<style scoped>
/* 字段与「浏览…」底部对齐：TextField 的根是纵向 flex，直接并排会按顶部对齐。 */
.picker {
  display: flex;
  align-items: flex-end;
  gap: var(--space-2);
}

.picker__field {
  flex: 1 1 auto;
  min-width: 0;
}
</style>
