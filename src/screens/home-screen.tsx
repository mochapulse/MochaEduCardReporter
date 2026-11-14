import {
  CircleHelp,
  FileSpreadsheet,
  FileX,
  FolderOpen,
  FolderX,
  Settings,
  Trash2,
} from 'lucide-react'
import { useEffect, useState } from 'react'
import { ScreenFrame } from '@/app/screen-frame'
import { Button } from '@/components/ui/button'
import { Card, CardTitle } from '@/components/ui/card'
import { ConfirmDialog } from '@/components/ui/dialog'
import { DropZone } from '@/components/ui/drop-zone'
import { EmptyState } from '@/components/ui/empty-state'
import { Notice } from '@/components/ui/notice'
import { t } from '@/i18n/es'
import type { Api, HistoryEntry } from '@/lib/api'
import { ErrorPanel } from '@/features/errors/error-panel'
import { describeRun } from '@/features/format'
import { formatDisplayDate } from '@/features/assistant/dates'
import { isSupportedGradebook } from '@/features/assistant/paths'
import type { AssistantActions } from '@/features/assistant/use-assistant'
import type { FlowState } from '@/features/assistant/state'
import { useHomeData } from '@/features/home/use-home-data'

export interface HomeScreenProps {
  api: Api
  state: FlowState
  actions: AssistantActions
}

export function HomeScreen({ api, state, actions }: HomeScreenProps) {
  const home = useHomeData(api)
  const [dragging, setDragging] = useState(false)
  const [rejected, setRejected] = useState(false)
  const [toRemove, setToRemove] = useState<HistoryEntry | null>(null)
  const opening = state.busy?.kind === 'opening'

  // Files dropped on the window: Tauri reports real paths (HTML5 drops in a webview do not).
  const { openFile } = actions
  useEffect(() => {
    let unlisten: (() => void) | null = null
    let alive = true
    api
      .onFileDrop((event) => {
        if (event.type === 'hover') setDragging(true)
        else if (event.type === 'leave') setDragging(false)
        else {
          setDragging(false)
          const supported = event.paths.find(isSupportedGradebook)
          setRejected(supported === undefined)
          if (supported !== undefined) void openFile(supported)
        }
      })
      .then((stop) => {
        if (alive) unlisten = stop
        else stop()
      })
      .catch(() => undefined)
    return () => {
      alive = false
      unlisten?.()
    }
  }, [api, openFile])

  return (
    <ScreenFrame
      title={t('home.heading')}
      windowPage={t('home.title')}
      subtitle={t('home.subtitle')}
      headerActions={
        <>
          <Button
            variant="ghost"
            icon={Settings}
            label={t('common.settings')}
            onClick={actions.openSettings}
          />
          <Button
            variant="ghost"
            icon={CircleHelp}
            label={t('common.help')}
            onClick={actions.openHelp}
          />
        </>
      }
    >
      <div className="flex flex-col gap-6">
        {state.error ? <ErrorPanel error={state.error} /> : null}
        {rejected ? <Notice variant="warning">{t('dropZone.rejected')}</Notice> : null}
        {opening ? <Notice variant="info">{t('home.opening')}</Notice> : null}

        <DropZone
          forceDragOver={dragging}
          onChooseFile={() => {
            setRejected(false)
            void actions.pickFile()
          }}
          onDropFiles={(accepted, others) => {
            setRejected(accepted.length === 0 && others.length > 0)
            // A browser drop has no file path; only the development mock can use it.
            if (accepted.length > 0 && api.kind === 'mock') void actions.pickFile()
          }}
        />

        <div className="grid gap-6 lg:grid-cols-2">
          <section aria-labelledby="recents-title">
            <h2 id="recents-title" className="mb-3 text-2xl font-semibold">
              {t('home.recents.title')}
            </h2>
            {home.recents.length === 0 ? (
              <EmptyState icon={FileSpreadsheet} title={t('empty.noRecents')} />
            ) : (
              <ul className="flex flex-col gap-3">
                {home.recents.map((file) => (
                  <li key={file.path}>
                    {file.exists === false ? (
                      <div className="flex items-start gap-3 rounded-md border-2 border-dashed border-fg-muted bg-surface p-3 text-fg-muted">
                        <FileX aria-hidden="true" className="mt-0.5 size-6 shrink-0" />
                        <div className="min-w-0">
                          <p className="font-semibold break-words">{file.file_name}</p>
                          <p>{t('home.recents.missing')}</p>
                        </div>
                      </div>
                    ) : (
                      <button
                        type="button"
                        onClick={() => void actions.openFile(file.path)}
                        disabled={opening}
                        className="flex min-h-11 w-full cursor-pointer items-start gap-3 rounded-md border-2 border-primary bg-surface p-3 text-left hover:bg-surface-muted disabled:cursor-not-allowed"
                      >
                        <FileSpreadsheet
                          aria-hidden="true"
                          className="mt-0.5 size-6 shrink-0 text-primary"
                        />
                        <span className="min-w-0">
                          <span className="block font-semibold break-words text-primary">
                            {file.file_name}
                          </span>
                          <span className="block text-fg-muted">
                            {t('home.recents.openedOn', {
                              date: formatDisplayDate(file.last_used),
                            })}
                          </span>
                        </span>
                      </button>
                    )}
                  </li>
                ))}
              </ul>
            )}
          </section>

          <section aria-labelledby="history-title">
            <h2 id="history-title" className="mb-3 text-2xl font-semibold">
              {t('home.history.title')}
            </h2>
            {home.history.length === 0 ? (
              <EmptyState icon={FolderX} title={t('empty.noHistory')} />
            ) : (
              <ul className="flex max-h-[28rem] flex-col gap-3 overflow-auto pr-1">
                {home.history.map((entry) => {
                  const title =
                    describeRun(entry.subject, entry.group, entry.period) ||
                    t('home.history.untitled')
                  return (
                    <li key={entry.id}>
                      <Card className="p-4">
                        <CardTitle level={3} className="break-words">
                          {title}
                        </CardTitle>
                        <p className="text-fg-muted">
                          {t('home.history.count', { count: entry.generated_count })}
                          {' · '}
                          {t('home.history.generatedOn', {
                            date: formatDisplayDate(entry.created_at),
                          })}
                        </p>
                        {entry.folder_exists === false ? (
                          <p className="mt-2 text-fg-muted">{t('home.history.folderMissing')}</p>
                        ) : null}
                        <div className="mt-3 flex flex-wrap gap-2">
                          {entry.folder_exists === false ? null : (
                            <Button
                              variant="outline"
                              icon={FolderOpen}
                              label={t('home.history.open')}
                              aria-label={`${t('home.history.open')}: ${title}`}
                              onClick={() => {
                                actions.openHistory(entry)
                              }}
                            />
                          )}
                          <Button
                            variant="ghost"
                            icon={Trash2}
                            label={t('home.history.remove')}
                            aria-label={t('home.history.removeLabel', { name: title })}
                            onClick={() => {
                              setToRemove(entry)
                            }}
                          />
                        </div>
                      </Card>
                    </li>
                  )
                })}
              </ul>
            )}
          </section>
        </div>
      </div>

      <ConfirmDialog
        open={toRemove !== null}
        onOpenChange={(open) => {
          if (!open) setToRemove(null)
        }}
        title={t('home.history.confirmRemove.title')}
        description={t('home.history.confirmRemove.description')}
        confirmLabel={t('home.history.confirmRemove.confirm')}
        cancelLabel={t('home.history.confirmRemove.keep')}
        destructive
        onConfirm={() => {
          if (toRemove) void home.removeHistoryEntry(toRemove.id).catch(() => undefined)
        }}
      />
    </ScreenFrame>
  )
}
