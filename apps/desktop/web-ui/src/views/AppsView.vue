<script setup lang="ts">
import { useAppsStore } from '@/stores/apps'
import { useServiceStore } from '@/stores/service'
import { useAppsCatalog } from '@/composables/useAppsCatalog'
import { useAppsFilters } from '@/composables/useAppsFilters'
import { useAppRowActions } from '@/composables/useAppRowActions'
import { useAppIcons } from '@/composables/useAppIcons'
import { useAppDetail } from '@/composables/useAppDetail'
import { useAppSelection } from '@/composables/useAppSelection'
import { useAppAdd } from '@/composables/useAppAdd'
import Card from '@/components/ui/Card.vue'
import Chip from '@/components/ui/Chip.vue'
import EmptyState from '@/components/ui/EmptyState.vue'
import LoadingBlock from '@/components/ui/LoadingBlock.vue'
import ErrorLine from '@/components/ui/ErrorLine.vue'
import Pagination from '@/components/ui/Pagination.vue'
import ServiceGate from '@/components/layout/ServiceGate.vue'
import AppToolbar from '@/components/apps/AppToolbar.vue'
import AppRow from '@/components/apps/AppRow.vue'
import AppDetailDrawer from '@/components/apps/AppDetailDrawer.vue'
import AppRenameModal from '@/components/apps/AppRenameModal.vue'
import AppIconModal from '@/components/apps/AppIconModal.vue'
import AppAddModal from '@/components/apps/AppAddModal.vue'

/**
 * 应用与游戏目录页面的装配层：目录数据、筛选、行内动作、图标、详情、批量与新增
 * 分别由对应 composable 承担，这里只做接线与模板绑定。
 */
const apps = useAppsStore()
const service = useServiceStore()

const catalog = useAppsCatalog()
const {
  kind,
  items,
  total,
  loading,
  error,
  scanBusy,
  scanError,
  page,
  pageCount,
  hasPrev,
  hasNext,
  rangeStart,
  rangeEnd,
  subtitle,
  runScan,
  reload,
} = catalog

const filters = useAppsFilters()
const { searchDraft, onKindChange, onIncludeHiddenChange, onFavoriteOnlyChange } = filters

const rows = useAppRowActions()
const {
  renameTarget,
  renameBusy,
  toggleFavorite,
  togglePinned,
  toggleHidden,
  removeApp,
  launchApp,
  restoreName,
  openRename,
  closeRename,
  submitRename,
} = rows

const icons = useAppIcons()
const { iconTarget, openIcon, closeIcon, onIconRefreshed } = icons

const detail = useAppDetail()
const { detailApp, openDetail, closeDetail } = detail

const selection = useAppSelection()
const { toggleSelect, selectAll, clearSelection, runBatch } = selection

const add = useAppAdd()
const { addOpen, addBusy, addError, openAdd, closeAdd, submitAdd } = add

/** 搜索框输入：只写筛选草稿，真正查询由防抖触发 */
function onQueryInput(value: string): void {
  filters.searchDraft.value = value
}

/** 空状态按钮：有关键词时清空搜索，否则触发重新扫描 */
function onEmptyAction(): void {
  if (apps.query) filters.clearSearch()
  else void catalog.runScan()
}
</script>

<template>
  <ServiceGate>
    <Card padding="none" :subtitle="subtitle">
      <template #actions>
        <Chip v-if="apps.isScanning" size="sm" tone="warning" dot>扫描进行中，完成后自动刷新</Chip>
      </template>

      <AppToolbar
        :kind="kind"
        :query="searchDraft"
        :include-hidden="apps.includeHidden"
        :favorite-only="apps.favoriteOnly"
        :hidden-visible-count="apps.hiddenCount"
        :stats="apps.stats"
        :scanning="apps.isScanning"
        :loading="loading"
        :scan-busy="scanBusy"
        :scan-error="scanError"
        :selected-count="apps.selectedCount"
        :batch-busy="apps.batchBusy"
        :can-add="service.canAddApp"
        @update:query="onQueryInput"
        @update:kind="onKindChange"
        @update:include-hidden="onIncludeHiddenChange"
        @update:favorite-only="onFavoriteOnlyChange"
        @scan="runScan"
        @reload="reload"
        @add="openAdd"
        @batch="runBatch"
        @select-all="selectAll"
        @clear-selection="clearSelection"
      />

      <LoadingBlock v-if="loading && items.length === 0" label="正在拉取目录…" />

      <EmptyState
        v-else-if="items.length === 0"
        :title="apps.query ? '没有匹配的条目' : '目录为空'"
        :description="
          apps.query
            ? '尝试更换关键词，或清空搜索查看全部条目。'
            : '可以先点「重新扫描」让服务端扫描本机已安装的应用。'
        "
        :action-label="apps.query ? '清空搜索' : '重新扫描'"
        @action="onEmptyAction"
      />

      <ul v-else class="apps__list">
        <AppRow
          v-for="app in items"
          :key="app.id"
          :app="app"
          :busy="apps.isBusy(app.id)"
          :selected="apps.isSelected(app.id)"
          :can-launch="service.canLaunchApp"
          :can-manage-icon="service.canManageIcon"
          @rename="openRename"
          @toggle-hidden="toggleHidden"
          @delete="removeApp"
          @launch="launchApp"
          @restore-name="restoreName"
          @toggle-select="toggleSelect"
          @icon="openIcon"
          @toggle-favorite="toggleFavorite"
          @toggle-pinned="togglePinned"
          @detail="openDetail"
        />
      </ul>

      <Pagination
        v-if="items.length > 0"
        :page="page"
        :page-count="pageCount"
        :total="total"
        :range-start="rangeStart"
        :range-end="rangeEnd"
        :has-prev="hasPrev"
        :has-next="hasNext"
        :loading="loading"
        unit="条"
        @prev="apps.prevPage()"
        @next="apps.nextPage()"
      />
    </Card>

    <AppRenameModal
      :app="renameTarget"
      :busy="renameBusy"
      @submit="submitRename"
      @close="closeRename"
    />

    <AppIconModal
      :app="iconTarget"
      :busy="apps.isBusy(iconTarget?.id ?? '')"
      @refreshed="onIconRefreshed"
      @close="closeIcon"
    />

    <AppDetailDrawer
      :app="detailApp"
      :busy="apps.isBusy(detailApp?.id ?? '')"
      :can-launch="service.canLaunchApp"
      :can-manage-icon="service.canManageIcon"
      @close="closeDetail"
      @launch="launchApp"
      @toggle-favorite="toggleFavorite"
      @toggle-pinned="togglePinned"
      @rename="openRename"
      @icon="openIcon"
      @toggle-hidden="toggleHidden"
      @restore-name="restoreName"
      @delete="removeApp"
    />

    <AppAddModal
      :open="addOpen"
      :busy="addBusy"
      :default-kind="apps.kind"
      :error="addError"
      @submit="submitAdd"
      @close="closeAdd"
    />
  </ServiceGate>

  <ErrorLine :error="error" />
</template>

<style scoped>
.apps__list {
  display: flex;
  flex-direction: column;
}
</style>
