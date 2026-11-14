import { lazy, Suspense, useEffect } from 'react'
import { useApi } from '@/lib/api'
import { useSettingsController } from '@/lib/settings'
import { useAssistant, type AssistantActions } from '@/features/assistant/use-assistant'
import type { FlowState, View } from '@/features/assistant/state'
import { GeneratingScreen } from '@/screens/generating-screen'
import { HelpScreen } from '@/screens/help-screen'
import { HomeScreen } from '@/screens/home-screen'
import { ResultsScreen } from '@/screens/results-screen'
import { ReviewScreen } from '@/screens/review-screen'
import { SaveScreen } from '@/screens/save-screen'
import { SettingsScreen } from '@/screens/settings-screen'

// The component gallery is a development tool: reachable with `?gallery`, never in a build.
const GalleryPage = import.meta.env.DEV
  ? lazy(() => import('@/gallery/gallery-page').then((m) => ({ default: m.GalleryPage })))
  : null

const BACK_WITH_ESCAPE: readonly View[] = ['review', 'save', 'results', 'settings', 'help']

export interface AppProps {
  /** Starting state (tests and development scenarios). */
  initialState?: FlowState
  /** Called once with the assistant's actions (development scenarios). */
  onReady?: (actions: AssistantActions) => void
}

export default function App({ initialState, onReady }: AppProps) {
  const api = useApi()
  const settings = useSettingsController()
  const { state, actions } = useAssistant(api, settings, initialState)

  useEffect(() => {
    onReady?.(actions)
    // Only once, when the assistant first exists.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [])

  // Esc goes back, Enter presses the primary action when nothing else owns the key.
  const { view } = state
  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.defaultPrevented) return
      if (event.key === 'Escape') {
        if (BACK_WITH_ESCAPE.includes(view)) actions.back()
        return
      }
      if (event.key === 'Enter') {
        const target = event.target
        const idle =
          target === document.body ||
          (target instanceof HTMLElement && target.hasAttribute('data-screen-heading'))
        if (!idle) return
        const primary = document.querySelector<HTMLButtonElement>('[data-primary-action]')
        if (primary && !primary.disabled) {
          event.preventDefault()
          primary.click()
        }
      }
    }
    window.addEventListener('keydown', onKeyDown)
    return () => {
      window.removeEventListener('keydown', onKeyDown)
    }
  }, [view, actions])

  if (GalleryPage && new URLSearchParams(window.location.search).has('gallery')) {
    return (
      <Suspense fallback={null}>
        <GalleryPage />
      </Suspense>
    )
  }

  const screenKey = `${state.view}:${state.reviewStep}`
  switch (state.view) {
    case 'home':
      return <HomeScreen key={screenKey} api={api} state={state} actions={actions} />
    case 'review':
      return <ReviewScreen key={screenKey} state={state} actions={actions} />
    case 'save':
      return <SaveScreen key={screenKey} state={state} actions={actions} />
    case 'generating':
      return <GeneratingScreen key={screenKey} state={state} actions={actions} />
    case 'results':
      return <ResultsScreen key={screenKey} state={state} actions={actions} />
    case 'settings':
      return <SettingsScreen key={screenKey} actions={actions} />
    case 'help':
      return <HelpScreen key={screenKey} actions={actions} />
  }
}
