import { ScrollText } from 'lucide-react'
import { useEffect, useState } from 'react'
import { ScreenFrame } from '@/app/screen-frame'
import { Button } from '@/components/ui/button'
import { Card, CardTitle } from '@/components/ui/card'
import { ChoiceGroup } from '@/components/ui/choice-group'
import { Notice } from '@/components/ui/notice'
import { PathField } from '@/components/ui/path-field'
import { t } from '@/i18n/es'
import { toAppError, useApi, type AppError, type AppInfo } from '@/lib/api'
import { useSettings } from '@/lib/settings'
import { textSizes, useTextSize, type TextSize } from '@/lib/text-size'
import { themePreferences, useTheme, type ThemePreference } from '@/lib/theme'
import { ErrorPanel } from '@/features/errors/error-panel'
import type { AssistantActions } from '@/features/assistant/use-assistant'

const textSizeLabels: Record<TextSize, string> = {
  normal: t('settings.textSize.normal'),
  grande: t('settings.textSize.large'),
  'muy-grande': t('settings.textSize.extraLarge'),
}
const themeLabels: Record<ThemePreference, string> = {
  claro: t('settings.theme.light'),
  oscuro: t('settings.theme.dark'),
  sistema: t('settings.theme.system'),
}

type UpdateState =
  | { readonly kind: 'idle' | 'checking' | 'current' }
  | { readonly kind: 'available'; readonly version: string }
  | { readonly kind: 'installing'; readonly version: string }

export function SettingsScreen({ actions }: { actions: AssistantActions }) {
  const api = useApi()
  const { settings, saveFailed, update } = useSettings()
  const { preference, setPreference } = useTheme()
  const { size, setSize } = useTextSize()
  const [defaultBase, setDefaultBase] = useState('')
  const [info, setInfo] = useState<AppInfo | null>(null)
  const [error, setError] = useState<AppError | null>(null)
  const [updateState, setUpdateState] = useState<UpdateState>({ kind: 'idle' })

  const customBase = settings.base_folder ?? null
  useEffect(() => {
    let alive = true
    api
      .defaultOutputBase()
      .then((base) => {
        if (alive) setDefaultBase(base)
      })
      .catch(() => undefined)
    api
      .appInfo()
      .then((value) => {
        if (alive) setInfo(value)
      })
      .catch(() => undefined)
    return () => {
      alive = false
    }
  }, [api, customBase])

  const run = async (action: () => Promise<void>) => {
    setError(null)
    try {
      await action()
    } catch (failure) {
      setError(toAppError(failure))
    }
  }

  const changeBase = () =>
    run(async () => {
      const picked = await api.pickFolder(customBase ?? defaultBase)
      if (picked !== null) update({ base_folder: picked })
    })

  const checkUpdate = async () => {
    setError(null)
    setUpdateState({ kind: 'checking' })
    try {
      const found = await api.checkForUpdate()
      setUpdateState(found ? { kind: 'available', version: found.version } : { kind: 'current' })
    } catch (failure) {
      setUpdateState({ kind: 'idle' })
      setError(toAppError(failure))
    }
  }

  const installUpdate = async (version: string) => {
    setError(null)
    setUpdateState({ kind: 'installing', version })
    try {
      await api.installUpdate()
    } catch (failure) {
      setUpdateState({ kind: 'available', version })
      setError(toAppError(failure))
    }
  }

  return (
    <ScreenFrame
      title={t('settings.title')}
      subtitle={t('settings.intro')}
      back={<Button variant="outline" label={t('common.back')} onClick={actions.back} />}
    >
      <div className="flex flex-col gap-5">
        {error ? <ErrorPanel error={error} /> : null}
        {saveFailed ? <Notice variant="warning">{t('settings.saveFailed')}</Notice> : null}

        <Card className="flex flex-col gap-3">
          <PathField
            label={t('settings.baseFolder')}
            path={customBase ?? defaultBase}
            onChange={() => void changeBase()}
          />
          <p className="text-fg-muted">{t('settings.baseFolderHelp')}</p>
          {customBase ? (
            <div>
              <Button
                variant="ghost"
                label={t('settings.useDefaultFolder')}
                onClick={() => {
                  update({ base_folder: null })
                }}
                className="-ml-5"
              />
            </div>
          ) : null}
        </Card>

        <Card>
          <ChoiceGroup<TextSize>
            legend={t('settings.textSize.title')}
            value={size}
            onValueChange={setSize}
            choices={textSizes.map((value) => ({ value, label: textSizeLabels[value] }))}
          />
        </Card>

        <Card>
          <ChoiceGroup<ThemePreference>
            legend={t('settings.theme.title')}
            value={preference}
            onValueChange={setPreference}
            choices={themePreferences.map((value) => ({ value, label: themeLabels[value] }))}
          />
        </Card>

        <Card className="flex flex-col gap-3">
          <CardTitle level={2}>{t('settings.updates.title')}</CardTitle>
          {updateState.kind === 'current' ? (
            <p className="text-fg-muted">{t('settings.updates.upToDate')}</p>
          ) : null}
          {updateState.kind === 'available' || updateState.kind === 'installing' ? (
            <p>
              {t('settings.updates.available', { version: updateState.version })}{' '}
              {updateState.kind === 'installing' ? t('settings.updates.restartHint') : null}
            </p>
          ) : null}
          <div>
            {updateState.kind === 'available' ? (
              <Button
                variant="outline"
                label={t('settings.updates.install')}
                onClick={() => void installUpdate(updateState.version)}
              />
            ) : (
              <Button
                variant="outline"
                label={
                  updateState.kind === 'checking'
                    ? t('settings.updates.checking')
                    : updateState.kind === 'installing'
                      ? t('settings.updates.installing', { version: updateState.version })
                      : t('settings.updates.check')
                }
                disabled={updateState.kind === 'checking' || updateState.kind === 'installing'}
                onClick={() => void checkUpdate()}
              />
            )}
          </div>
        </Card>

        <Card className="flex flex-col gap-3">
          <CardTitle level={2}>{t('settings.logs')}</CardTitle>
          <p className="text-fg-muted">{t('settings.logsHelp')}</p>
          <div>
            <Button
              variant="outline"
              icon={ScrollText}
              label={t('settings.openLogs')}
              onClick={() => void run(() => api.openLogDir())}
            />
          </div>
        </Card>

        {info ? (
          <p className="text-fg-muted">{t('app.version', { version: info.version })}</p>
        ) : null}
      </div>
    </ScreenFrame>
  )
}
