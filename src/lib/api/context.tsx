import { isTauri } from '@tauri-apps/api/core'
import { createContext, useContext, type ReactNode } from 'react'
import { createMockApi } from './mock-api'
import { createTauriApi } from './tauri-api'
import type { Api } from './types'

/** Real API inside the Tauri window, in-memory mock in a plain browser. */
export function createDefaultApi(): Api {
  return isTauri() ? createTauriApi() : createMockApi()
}

const ApiContext = createContext<Api | null>(null)

export function ApiProvider({ api, children }: { api: Api; children: ReactNode }) {
  return <ApiContext.Provider value={api}>{children}</ApiContext.Provider>
}

export function useApi(): Api {
  const api = useContext(ApiContext)
  if (!api) throw new Error('useApi must be used inside <ApiProvider>')
  return api
}
