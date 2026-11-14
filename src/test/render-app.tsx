import { render } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { AppProviders } from '@/app/providers'
import type { FlowState } from '@/features/assistant/state'
import App from '@/App'
import { createMockApi, type MockApi } from '@/lib/api'
import { createSettingsController, type SettingsController } from '@/lib/preferences'

export interface RenderedApp {
  user: ReturnType<typeof userEvent.setup>
  api: MockApi
  settings: SettingsController
  container: HTMLElement
}

/** Renders the whole app against the mock API (or one the test supplies). */
export function renderApp(options: { api?: MockApi; initialState?: FlowState } = {}): RenderedApp {
  const api = options.api ?? createMockApi({ stepDelayMs: 0 })
  const settings = createSettingsController(
    { base_folder: null, remember_base_folder: true, text_size: 'normal', theme: 'claro' },
    (value) => api.saveSettings({ ...value }),
  )
  const user = userEvent.setup()
  const { container } = render(
    <AppProviders api={api} settings={settings}>
      <App initialState={options.initialState} />
    </AppProviders>,
  )
  return { user, api, settings, container }
}
