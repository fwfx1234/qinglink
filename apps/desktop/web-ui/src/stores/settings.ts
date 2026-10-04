import { computed, ref } from 'vue'
import { defineStore } from 'pinia'
import { settingsApi } from '@/api/endpoints'
import type { SettingsData, SettingsPatch } from '@/api/types'
import { toAppError, type AppError } from '@/utils/errors'

/** 桌面端设置（§4.3 / §4.4） */
export const useSettingsStore = defineStore('settings', () => {
  const settings = ref<SettingsData | null>(null)
  const loading = ref(false)
  const saving = ref(false)
  const error = ref<AppError | null>(null)
  /** 正在保存的字段名 */
  const savingKey = ref<keyof SettingsData | null>(null)

  const autostart = computed(() => settings.value?.autostart ?? false)
  const keepAwake = computed(() => settings.value?.keep_awake ?? false)
  const debugLog = computed(() => settings.value?.debug_log ?? false)
  const loaded = computed(() => settings.value !== null)

  function apply(next: SettingsData): void {
    settings.value = next
  }

  async function load(): Promise<void> {
    loading.value = true
    try {
      apply(await settingsApi.get())
      error.value = null
    } catch (caught) {
      error.value = toAppError(caught)
    } finally {
      loading.value = false
    }
  }

  /** §4.4 字段均可选，只传需要修改的；返回修改后的完整设置 */
  async function update(patch: SettingsPatch, key: keyof SettingsData | null = null): Promise<void> {
    saving.value = true
    savingKey.value = key
    try {
      apply(await settingsApi.update(patch))
      error.value = null
    } catch (caught) {
      error.value = toAppError(caught)
      throw caught
    } finally {
      saving.value = false
      savingKey.value = null
    }
  }

  async function toggle(key: keyof SettingsData, value: boolean): Promise<void> {
    await update({ [key]: value } as SettingsPatch, key)
  }

  return {
    settings,
    loading,
    saving,
    error,
    savingKey,
    autostart,
    keepAwake,
    debugLog,
    loaded,
    apply,
    load,
    update,
    toggle,
  }
})
