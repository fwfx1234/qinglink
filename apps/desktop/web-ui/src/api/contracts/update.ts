/**
 * 程序更新状态契约（docs/web-api.md §4.37 / §5 update_state）。
 * 由后端 update_state 事件与 GET /update 共用同一份结构。
 */

/** 更新流程阶段，与后端 UpdatePhase 的 snake_case 序列化值一致。 */
export type UpdatePhase =
  | 'idle'
  | 'checking'
  | 'up_to_date'
  | 'available'
  | 'downloading'
  | 'ready'
  | 'installing'
  | 'error'

export interface UpdateView {
  phase: UpdatePhase
  current_version: string
  latest_version: string | null
  release_url: string | null
  notes: string
  progress: number
  downloaded_bytes: number
  total_bytes: number
  error: string | null
  busy: boolean
  last_check_secs: number | null
}
