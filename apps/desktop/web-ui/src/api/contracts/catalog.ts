/**
 * 应用目录与图标（docs/web-api.md §4.14 - §4.20、§4.29 - §4.33）。
 *
 * 目录条目、统计、改名 / 隐藏 / 常用 / 置顶 / 删除 / 扫描结果、新增与启动、
 * 批量操作，以及图标的候选列表与重解析结果都在这里。
 */

import type { Page } from './envelope'

/* ------------------------------------------------------------------ */
/* §4.14 - §4.20 应用目录                                               */
/* ------------------------------------------------------------------ */

export type CatalogKind = 'app' | 'game'

export interface AppItem {
  id: string
  /** 当前生效的显示名 */
  name: string
  original_name: string
  exe_path: string
  category: string
  source: string
  hidden: boolean
  user_added: boolean
  args: string[]
  has_icon: boolean
  /** name != original_name */
  renamed: boolean
  /**
   * 图标来源：manual / steam / shortcut / executable / image / shell / none。
   * 用于解释"图标看起来不对"，也是换图标弹窗的默认提示。
   */
  icon_source?: string | null
  /** 图标最后一次解析的 Unix 秒；作为图标 URL 的缓存版本号 */
  icon_updated_at?: number | null
  /** 在「常用（我的库）」中；服务端拥有，手机端看到的是同一份 */
  favorite: boolean
  /** 置顶到手机首页 */
  pinned: boolean
}

/** §4.32 单个条目的图标候选 */
export interface AppIconCandidate {
  path: string
  /** executable | shortcut | shortcut_icon | image */
  kind: string
  size_bytes: number
}

export interface AppIconCandidates {
  id: string
  candidates: AppIconCandidate[]
}

/** §4.33 重新解析图标的结果 */
export interface AppIconUpdate {
  id: string
  icon_base64?: string | null
  source: string
  icon_path?: string | null
}

export interface AppPage extends Page<AppItem> {
  kind: CatalogKind
}

export interface CatalogKindStats {
  total: number
  hidden: number
  user_added: number
  renamed: number
  /** 其中属于「常用」的条数 */
  favorite: number
}

export interface AppsStats {
  app: CatalogKindStats
  game: CatalogKindStats
}

export interface RenameResult {
  id: string
  name: string
  /** name != original_name，后端会一并返回 */
  renamed?: boolean
}

export interface HiddenResult {
  id: string
  hidden: boolean
}

export interface FavoriteResult {
  id: string
  favorite: boolean
}

export interface PinnedResult {
  id: string
  pinned: boolean
}

export interface DeleteResult {
  deleted: boolean
  /** 被软删除的条目 id */
  id?: string
}

export interface ScanResult {
  accepted: boolean
  kind: CatalogKind
}

/** §4.29 新增自定义条目 */
export interface AppAddRequest {
  path: string
  name: string
  kind: CatalogKind
  args: string[]
}

export interface AppAddResult {
  id: string
  name: string
  kind: CatalogKind
}

/** §4.30 启动条目 */
export interface AppLaunchResult {
  launched: boolean
  id: string
}

/** §4.31 批量操作 */
export type AppBatchAction = 'hide' | 'show' | 'delete' | 'restore_name' | 'refresh_icon'

export interface AppBatchRequest {
  action: AppBatchAction
  ids: string[]
}

export interface AppBatchItemResult {
  id: string
  ok: boolean
  code?: string
  message?: string
}

export interface AppBatchResult {
  results: AppBatchItemResult[]
  succeeded: number
  failed: number
}
