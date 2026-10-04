<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import type { AppIconCandidate, AppItem } from '@/api/types'
import { dialogApi } from '@/api/endpoints'
import { useAppsStore } from '@/stores/apps'
import { toAppError, type AppError } from '@/utils/errors'
import { shortenPath } from '@/utils/format'
import Modal from '@/components/ui/Modal.vue'
import Button from '@/components/ui/Button.vue'
import ErrorLine from '@/components/ui/ErrorLine.vue'
import InfoTip from '@/components/ui/InfoTip.vue'
import Spinner from '@/components/ui/Spinner.vue'
import AppIcon from './AppIcon.vue'

/** 解析顺序说明：太长，默认收进 tooltip，不占版面。 */
const RESOLVE_CHAIN_TIP =
  '依次查找：快捷方式指定的图标（含图标序号）→ 程序内嵌资源 → 同目录 icon.png / 同名 .png、.ico → 系统默认图标。Steam 条目优先用官方封面。'

const MANUAL_TIP =
  '可以直接粘贴路径，也可以点「浏览…」在弹出的窗口里选（支持 .png / .jpg / .ico 以及 .exe）。选中的文件会记为该条目的首选图标，直到再次「自动重新扫描」清除。'

const props = defineProps<{
  app: AppItem | null
  busy: boolean
}>()

const emit = defineEmits<{
  (event: 'close'): void
  /** 图标已重新解析，父组件的图标版本号需要跟着变 */
  (event: 'refreshed', id: string): void
}>()

const apps = useAppsStore()
const candidates = ref<AppIconCandidate[]>([])
const candidatesBusy = ref(false)
const candidatesError = ref<string | null>(null)
const manualPath = ref('')
const localError = ref<AppError | null>(null)
const localBusy = ref(false)
const picking = ref(false)

/** 本地解析进度覆盖 store 的 busy，弹窗里需要独立反馈 */
const working = computed(() => props.busy || localBusy.value)

const sourceLabel = computed(() => {
  const source = props.app?.icon_source
  if (!source) return '尚未解析（列表按需加载时会自动解析）'
  const labels: Record<string, string> = {
    manual: '手动指定',
    steam: 'Steam 官方封面',
    shortcut: '快捷方式目标',
    executable: '程序内嵌资源',
    image: '同目录图片文件',
    shell: '系统默认图标（可能不准确）',
    none: '未找到可用图标',
  }
  return labels[source] ?? source
})

const kindLabel: Record<string, string> = {
  executable: '程序本体',
  shortcut: '快捷方式',
  shortcut_icon: '快捷方式指定图标',
  image: '图片文件',
}

/** 原生选择窗口的起始目录：程序自己所在目录最可能放着它的图标。 */
const startIn = computed(() => {
  const path = props.app?.exe_path ?? ''
  const index = path.lastIndexOf('\\')
  return index > 0 ? path.slice(0, index) : null
})

function formatBytes(bytes: number): string {
  if (bytes >= 1024 * 1024) return `${(bytes / 1024 / 1024).toFixed(1)} MB`
  if (bytes >= 1024) return `${Math.round(bytes / 1024)} KB`
  return `${bytes} B`
}

watch(
  () => props.app?.id,
  (id) => {
    candidates.value = []
    candidatesError.value = null
    manualPath.value = ''
    localError.value = null
    if (id) void loadCandidates(id)
  },
  { immediate: true },
)

async function loadCandidates(id: string): Promise<void> {
  candidatesBusy.value = true
  candidatesError.value = null
  try {
    const result = await apps.iconCandidates(id)
    candidates.value = result.candidates
  } catch (caught) {
    candidatesError.value = toAppError(caught).message
  } finally {
    candidatesBusy.value = false
  }
}

async function run(action: () => Promise<unknown>, success: string): Promise<void> {
  const app = props.app
  if (!app) return
  localBusy.value = true
  localError.value = null
  try {
    await action()
    emit('refreshed', app.id)
    manualPath.value = ''
    await loadCandidates(app.id)
    localError.value = null
    void success
  } catch (caught) {
    localError.value = toAppError(caught)
  } finally {
    localBusy.value = false
  }
}

function autoRefresh(): void {
  void run(() => apps.refreshIcon(props.app!.id, null), '已重新扫描')
}

function useCandidate(candidate: AppIconCandidate): void {
  void run(() => apps.refreshIcon(props.app!.id, candidate.path), '已应用所选图标')
}

function useManual(): void {
  const path = manualPath.value.trim()
  if (!path) return
  void run(() => apps.refreshIcon(props.app!.id, path), '已应用手动路径')
}

/**
 * 在 PC 上弹出原生选择窗口。
 *
 * 浏览器读不到本机文件路径，所以这一步必须由桌面端代为选择；选完直接应用，
 * 用户不必再回来点一次「应用」。
 */
async function browse(): Promise<void> {
  const app = props.app
  if (!app || picking.value) return
  picking.value = true
  localError.value = null
  try {
    const result = await dialogApi.pickFile('image', startIn.value)
    if (result.path) {
      manualPath.value = result.path
      await run(() => apps.refreshIcon(app.id, result.path), '已应用所选图标')
    }
  } catch (caught) {
    localError.value = toAppError(caught)
  } finally {
    picking.value = false
  }
}
</script>

<template>
  <Modal :open="app !== null" title="图标" @close="emit('close')">
    <div v-if="app" class="icon-modal">
      <div class="icon-modal__head">
        <AppIcon
          :id="app.id"
          :has-icon="app.has_icon"
          :version="app.icon_updated_at"
          :name="app.name"
          :size="64"
        />
        <div class="icon-modal__meta">
          <p class="icon-modal__name">{{ app.name || '（未命名）' }}</p>
          <p class="icon-modal__source">来源：{{ sourceLabel }}</p>
          <p class="icon-modal__path" :title="app.exe_path || app.id">
            {{ shortenPath(app.exe_path || app.id, 72) }}
          </p>
        </div>
      </div>

      <div class="icon-modal__actions">
        <Button size="sm" variant="primary" :loading="working" @click="autoRefresh">自动重新扫描</Button>
        <InfoTip :text="RESOLVE_CHAIN_TIP" label="按什么顺序解析？" />
        <Spinner v-if="working" :size="12" tone="muted" />
      </div>

      <ErrorLine v-if="localError" :error="localError" />

      <section class="icon-modal__section">
        <div class="icon-modal__section-head">
          <span>检测到的候选图标文件</span>
          <Spinner v-if="candidatesBusy" :size="12" tone="muted" />
        </div>
        <p v-if="candidatesError" class="icon-modal__error">{{ candidatesError }}</p>
        <p v-else-if="!candidatesBusy && candidates.length === 0" class="icon-modal__empty">
          没有在这台 PC 上找到可用的图标文件，可以在下面手动指定一张图片。
        </p>
        <ul v-else class="icon-modal__list">
          <li v-for="candidate in candidates" :key="candidate.path" class="icon-modal__row">
            <span class="icon-modal__kind">{{ kindLabel[candidate.kind] ?? candidate.kind }}</span>
            <span class="icon-modal__file" :title="candidate.path">{{ candidate.path }}</span>
            <span class="icon-modal__size">{{ formatBytes(candidate.size_bytes) }}</span>
            <Button size="sm" variant="secondary" :disabled="working" @click="useCandidate(candidate)">
              使用
            </Button>
          </li>
        </ul>
      </section>

      <section class="icon-modal__section">
        <div class="icon-modal__section-head">
          <span>使用其他图片或程序</span>
          <InfoTip :text="MANUAL_TIP" />
        </div>
        <div class="icon-modal__manual">
          <input
            v-model="manualPath"
            class="icon-modal__input"
            type="text"
            placeholder="D:\Games\Example\icon.png"
            spellcheck="false"
            @keyup.enter="useManual"
          />
          <Button size="sm" variant="secondary" :loading="picking" @click="browse">浏览…</Button>
          <Button size="sm" variant="secondary" :disabled="working || manualPath.trim().length === 0" @click="useManual">
            应用
          </Button>
        </div>
      </section>
    </div>
  </Modal>
</template>

<style scoped>
.icon-modal {
  display: flex;
  flex-direction: column;
  gap: var(--space-3);
}

.icon-modal__head {
  display: flex;
  align-items: center;
  gap: var(--space-3);
}

.icon-modal__meta {
  min-width: 0;
}

.icon-modal__name {
  font-size: var(--font-md);
  color: var(--text-1);
}

.icon-modal__source,
.icon-modal__path {
  font-size: var(--font-sm);
  color: var(--text-2);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.icon-modal__path {
  font-family: var(--font-mono);
  color: var(--text-3);
}

.icon-modal__actions {
  display: flex;
  align-items: center;
  gap: var(--space-3);
}

.icon-modal__section {
  border-top: 1px solid var(--border-1);
  padding-top: var(--space-4);
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
}

.icon-modal__section-head {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  font-size: var(--font-sm);
  font-weight: var(--weight-medium);
  color: var(--text-2);
}

.icon-modal__list {
  display: flex;
  flex-direction: column;
  gap: var(--space-1);
  max-height: 220px;
  overflow: auto;
}

.icon-modal__row {
  display: flex;
  align-items: center;
  gap: var(--space-3);
  padding: var(--space-2) var(--space-3);
  border-radius: var(--radius-md);
  background: var(--surface-2);
}

.icon-modal__kind {
  flex: none;
  font-size: var(--font-xs);
  color: var(--text-3);
  min-width: 96px;
}

.icon-modal__file {
  flex: 1 1 auto;
  min-width: 0;
  font-family: var(--font-mono);
  font-size: var(--font-xs);
  color: var(--text-1);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.icon-modal__size {
  flex: none;
  font-size: var(--font-xs);
  color: var(--text-3);
}

.icon-modal__manual {
  display: flex;
  flex-wrap: wrap;
  gap: var(--space-2);
}

.icon-modal__input {
  flex: 1 1 240px;
  min-width: 0;
  height: var(--control-height);
  padding: 0 var(--space-3);
  border-radius: var(--radius-md);
  border: 1px solid var(--border-1);
  background: var(--surface-1);
  color: var(--text-1);
  outline: none;
  transition: border-color var(--transition-fast), box-shadow var(--transition-fast);
}

.icon-modal__input::placeholder {
  color: var(--text-3);
}

.icon-modal__input:hover {
  border-color: var(--border-2);
}

.icon-modal__input:focus {
  border-color: var(--accent);
  box-shadow: 0 0 0 3px var(--accent-ring);
}

.icon-modal__empty,
.icon-modal__error {
  font-size: var(--font-sm);
  color: var(--text-3);
}

.icon-modal__error {
  color: var(--error);
}
</style>
