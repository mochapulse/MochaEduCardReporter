import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import { AppProviders } from '@/app/providers'
import { loadSettings } from '@/app/bootstrap'
import { createDefaultApi } from '@/lib/api'
import App from './App'
import './styles/globals.css'

const root = document.getElementById('root')
if (!root) throw new Error('Missing #root element')

async function start(container: HTMLElement) {
  // Development only: `?demo=<name>` opens a screen in a given state with the mock API.
  const demo = import.meta.env.DEV
    ? await import('@/app/dev-scenarios').then((m) =>
        m.createScenario(new URLSearchParams(window.location.search)),
      )
    : null
  const api = demo?.api ?? createDefaultApi()
  const settings = await loadSettings(api)
  createRoot(container).render(
    <StrictMode>
      <AppProviders api={api} settings={settings}>
        <App initialState={demo?.initial} onReady={demo?.autoplay} />
      </AppProviders>
    </StrictMode>,
  )
}

void start(root)
