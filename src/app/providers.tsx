import type { ReactNode } from 'react'
import { TooltipProvider } from '@/components/ui/tooltip'
import { ApiProvider, type Api } from '@/lib/api'
import type { SettingsController } from '@/lib/preferences'
import { SettingsProvider } from '@/lib/settings'
import { TextSizeProvider } from '@/lib/text-size'
import { ThemeProvider } from '@/lib/theme'

/** Everything the screens read from context: API, settings, theme, text size, tooltips. */
export function AppProviders({
  api,
  settings,
  children,
}: {
  api: Api
  settings: SettingsController
  children: ReactNode
}) {
  return (
    <ApiProvider api={api}>
      <SettingsProvider controller={settings}>
        <ThemeProvider store={settings}>
          <TextSizeProvider store={settings}>
            <TooltipProvider delayDuration={300}>{children}</TooltipProvider>
          </TextSizeProvider>
        </ThemeProvider>
      </SettingsProvider>
    </ApiProvider>
  )
}
