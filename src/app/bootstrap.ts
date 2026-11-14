import type { Api, Settings } from '@/lib/api'
import { createLocalStorageStore, createSettingsController } from '@/lib/preferences'

const DEFAULT_SETTINGS: Settings = {
  base_folder: null,
  remember_base_folder: true,
  text_size: 'normal',
  theme: 'claro',
}

/**
 * Reads the saved settings from the backend (falling back to defaults when it
 * cannot be reached), applies theme and text size to the document right away
 * and returns the controller that keeps saving later changes.
 */
export async function loadSettings(api: Api) {
  let initial: Settings = DEFAULT_SETTINGS
  try {
    initial = { ...DEFAULT_SETTINGS, ...(await api.getSettings()) }
  } catch {
    // The window must open even when the settings cannot be read.
  }
  const root = document.documentElement
  const dark =
    initial.theme === 'oscuro' ||
    (initial.theme === 'sistema' && window.matchMedia('(prefers-color-scheme: dark)').matches)
  root.dataset['theme'] = dark ? 'dark' : 'light'
  root.dataset['textSize'] = initial.text_size ?? 'normal'
  return createSettingsController(
    initial,
    (settings) => api.saveSettings(toSettings(settings)),
    createLocalStorageStore(),
  )
}

function toSettings(settings: Parameters<typeof createSettingsController>[0]): Settings {
  return { ...DEFAULT_SETTINGS, ...settings }
}
