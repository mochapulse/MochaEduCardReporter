import { createContext, useContext, useSyncExternalStore, type ReactNode } from 'react'
import type { PersistedSettings, SettingsController } from './preferences'

const SettingsContext = createContext<SettingsController | null>(null)

export function SettingsProvider({
  controller,
  children,
}: {
  controller: SettingsController
  children: ReactNode
}) {
  return <SettingsContext.Provider value={controller}>{children}</SettingsContext.Provider>
}

export function useSettingsController(): SettingsController {
  const controller = useContext(SettingsContext)
  if (!controller) throw new Error('useSettings must be used inside <SettingsProvider>')
  return controller
}

/** Live view of the persisted settings. */
export function useSettings(): {
  settings: PersistedSettings
  saveFailed: boolean
  update: (patch: PersistedSettings) => void
} {
  const controller = useSettingsController()
  const settings = useSyncExternalStore(controller.subscribe, controller.current)
  const saveFailed = useSyncExternalStore(controller.subscribe, controller.saveFailed)
  return { settings, saveFailed, update: controller.update }
}
