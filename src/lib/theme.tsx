import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useState,
  type ReactNode,
} from 'react'
import { readChoice, type PreferencesStore } from './preferences'

export const themePreferences = ['claro', 'oscuro', 'sistema'] as const
export type ThemePreference = (typeof themePreferences)[number]
export type ResolvedTheme = 'light' | 'dark'

const STORAGE_KEY = 'theme'
const DARK_QUERY = '(prefers-color-scheme: dark)'

interface ThemeContextValue {
  readonly preference: ThemePreference
  readonly resolved: ResolvedTheme
  readonly setPreference: (preference: ThemePreference) => void
}

const ThemeContext = createContext<ThemeContextValue | null>(null)

function systemTheme(): ResolvedTheme {
  return window.matchMedia(DARK_QUERY).matches ? 'dark' : 'light'
}

export function ThemeProvider({
  store,
  children,
}: {
  store: PreferencesStore
  children: ReactNode
}) {
  const [preference, setPreferenceState] = useState<ThemePreference>(() =>
    readChoice(store, STORAGE_KEY, themePreferences, 'claro'),
  )
  const [system, setSystem] = useState<ResolvedTheme>(systemTheme)

  useEffect(() => {
    const query = window.matchMedia(DARK_QUERY)
    const onChange = () => {
      setSystem(query.matches ? 'dark' : 'light')
    }
    query.addEventListener('change', onChange)
    return () => {
      query.removeEventListener('change', onChange)
    }
  }, [])

  const resolved: ResolvedTheme =
    preference === 'sistema' ? system : preference === 'oscuro' ? 'dark' : 'light'

  useEffect(() => {
    document.documentElement.dataset['theme'] = resolved
  }, [resolved])

  const setPreference = useCallback(
    (next: ThemePreference) => {
      setPreferenceState(next)
      store.set(STORAGE_KEY, next)
    },
    [store],
  )

  const value = useMemo(
    () => ({ preference, resolved, setPreference }),
    [preference, resolved, setPreference],
  )
  return <ThemeContext.Provider value={value}>{children}</ThemeContext.Provider>
}

export function useTheme(): ThemeContextValue {
  const context = useContext(ThemeContext)
  if (!context) throw new Error('useTheme must be used inside <ThemeProvider>')
  return context
}
