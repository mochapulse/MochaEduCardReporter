import { useCallback, useEffect, useState } from 'react'
import type { Api, HistoryEntry, RecentFile } from '@/lib/api'

export interface HomeData {
  loaded: boolean
  recents: readonly RecentFile[]
  history: readonly HistoryEntry[]
  removeHistoryEntry: (id: string) => Promise<void>
}

/** Recent files (last 5) and past generations for the start screen. */
export function useHomeData(api: Api): HomeData {
  const [data, setData] = useState<{
    loaded: boolean
    recents: readonly RecentFile[]
    history: readonly HistoryEntry[]
  }>({ loaded: false, recents: [], history: [] })

  useEffect(() => {
    let alive = true
    // The lists are a convenience: if they cannot be read, the screen shows them empty.
    Promise.all([
      api.getRecentFiles().catch((): RecentFile[] => []),
      api.getHistory().catch((): HistoryEntry[] => []),
    ])
      .then(([recents, history]) => {
        if (alive) setData({ loaded: true, recents: recents.slice(0, 5), history })
      })
      .catch(() => undefined)
    return () => {
      alive = false
    }
  }, [api])

  const removeHistoryEntry = useCallback(
    async (id: string) => {
      await api.removeHistoryEntry(id)
      setData((current) => ({
        ...current,
        history: current.history.filter((entry) => entry.id !== id),
      }))
    },
    [api],
  )

  return { ...data, removeHistoryEntry }
}
