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

export const textSizes = ['normal', 'grande', 'muy-grande'] as const
export type TextSize = (typeof textSizes)[number]

/** Root font-size multiplier per text size (applied in tokens.css via data-text-size). */
export const textSizeScale: Readonly<Record<TextSize, number>> = {
  normal: 1,
  grande: 1.125,
  'muy-grande': 1.25,
}

const STORAGE_KEY = 'text-size'

interface TextSizeContextValue {
  readonly size: TextSize
  readonly setSize: (size: TextSize) => void
}

const TextSizeContext = createContext<TextSizeContextValue | null>(null)

export function TextSizeProvider({
  store,
  children,
}: {
  store: PreferencesStore
  children: ReactNode
}) {
  const [size, setSizeState] = useState<TextSize>(() =>
    readChoice(store, STORAGE_KEY, textSizes, 'normal'),
  )

  useEffect(() => {
    document.documentElement.dataset['textSize'] = size
  }, [size])

  const setSize = useCallback(
    (next: TextSize) => {
      setSizeState(next)
      store.set(STORAGE_KEY, next)
    },
    [store],
  )

  const value = useMemo(() => ({ size, setSize }), [size, setSize])
  return <TextSizeContext.Provider value={value}>{children}</TextSizeContext.Provider>
}

export function useTextSize(): TextSizeContextValue {
  const context = useContext(TextSizeContext)
  if (!context) throw new Error('useTextSize must be used inside <TextSizeProvider>')
  return context
}
