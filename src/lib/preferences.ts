/**
 * Small persistence abstraction for user preferences (theme, text size, ...).
 *
 * The interface is synchronous on purpose so providers can render the right
 * theme on the first paint. A Tauri-store implementation will preload its
 * values into memory before mounting the app and implement this same interface.
 */
export interface PreferencesStore {
  get(key: string): string | null
  set(key: string, value: string): void
}

/** Volatile store: used in tests and as the fallback when storage is unavailable. */
export function createMemoryStore(
  initial: Readonly<Record<string, string>> = {},
): PreferencesStore {
  const data = new Map<string, string>(Object.entries(initial))
  return {
    get: (key) => data.get(key) ?? null,
    set: (key, value) => {
      data.set(key, value)
    },
  }
}

/**
 * localStorage-backed store. Every access is wrapped in try/catch because
 * storage can be blocked or throw (private windows, cleared site data).
 * Failed writes keep the value in memory for the rest of the session.
 */
export function createLocalStorageStore(prefix = 'mocha.'): PreferencesStore {
  const fallback = createMemoryStore()
  return {
    get(key) {
      try {
        return window.localStorage.getItem(prefix + key) ?? fallback.get(key)
      } catch {
        return fallback.get(key)
      }
    },
    set(key, value) {
      fallback.set(key, value)
      try {
        window.localStorage.setItem(prefix + key, value)
      } catch {
        // Storage unavailable: the in-memory fallback already holds the value.
      }
    },
  }
}

/** Reads a preference and validates it against the allowed values. */
export function readChoice<T extends string>(
  store: PreferencesStore,
  key: string,
  allowed: readonly T[],
  fallback: T,
): T {
  const value = store.get(key)
  return allowed.find((candidate) => candidate === value) ?? fallback
}

/** The settings the backend persists (a structural copy of the generated `Settings`). */
export interface PersistedSettings {
  base_folder?: string | null
  remember_base_folder?: boolean
  text_size?: 'normal' | 'grande' | 'muy-grande'
  theme?: 'claro' | 'oscuro' | 'sistema'
}

/**
 * Preferences backed by the application settings (saved through the backend).
 * It is also a {@link PreferencesStore}, so the theme and text-size providers
 * keep working unchanged: key `theme` maps to `theme`, `text-size` to
 * `text_size`. Writes are serialized and always send the latest full settings,
 * so two quick changes never overwrite each other with stale values.
 */
export interface SettingsController extends PreferencesStore {
  current: () => PersistedSettings
  update: (patch: PersistedSettings) => void
  subscribe: (listener: () => void) => () => void
  /** True when the last attempt to save failed. */
  saveFailed: () => boolean
  /** Resolves when every queued save has finished. */
  flush: () => Promise<void>
}

const THEMES = ['claro', 'oscuro', 'sistema'] as const
const TEXT_SIZES = ['normal', 'grande', 'muy-grande'] as const

export function createSettingsController(
  initial: PersistedSettings,
  save: (settings: PersistedSettings) => Promise<void>,
  /** Optional synchronous mirror (localStorage) read by `public/boot.js` before first paint. */
  mirror?: PreferencesStore,
): SettingsController {
  let settings: PersistedSettings = { ...initial }
  let failed = false
  let queue: Promise<void> = Promise.resolve()
  const listeners = new Set<() => void>()
  const notify = () => {
    for (const listener of listeners) listener()
  }
  const mirrorAll = () => {
    if (!mirror) return
    mirror.set('theme', settings.theme ?? 'claro')
    mirror.set('text-size', settings.text_size ?? 'normal')
  }
  mirrorAll()

  const update = (patch: PersistedSettings) => {
    settings = { ...settings, ...patch }
    mirrorAll()
    notify()
    queue = queue.then(async () => {
      try {
        await save(settings)
        failed = false
      } catch {
        failed = true
      }
      notify()
    })
  }

  return {
    current: () => settings,
    update,
    subscribe: (listener) => {
      listeners.add(listener)
      return () => {
        listeners.delete(listener)
      }
    },
    saveFailed: () => failed,
    flush: () => queue,
    get(key) {
      if (key === 'theme') return settings.theme ?? null
      if (key === 'text-size') return settings.text_size ?? null
      return null
    },
    set(key, value) {
      if (key === 'theme') {
        const theme = THEMES.find((candidate) => candidate === value)
        if (theme) update({ theme })
      } else if (key === 'text-size') {
        const size = TEXT_SIZES.find((candidate) => candidate === value)
        if (size) update({ text_size: size })
      }
    },
  }
}
