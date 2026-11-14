import { ExternalLink, FolderOpen, Printer, RefreshCw } from 'lucide-react'
import { useCallback, useMemo, useState } from 'react'
import { ScreenFrame } from '@/app/screen-frame'
import { Button } from '@/components/ui/button'
import { EmptyState } from '@/components/ui/empty-state'
import { Notice } from '@/components/ui/notice'
import { SearchField } from '@/components/ui/search-field'
import { PdfViewer } from '@/components/pdf-viewer'
import { getErrorMessage } from '@/i18n/errors'
import { t } from '@/i18n/es'
import { toAppError, useApi, type AppError, type GeneratedFile } from '@/lib/api'
import { cn } from '@/lib/utils'
import type { FlowState } from '@/features/assistant/state'
import type { AssistantActions } from '@/features/assistant/use-assistant'
import { ErrorPanel, TechnicalDetails } from '@/features/errors/error-panel'
import { describeRun } from '@/features/format'

export interface ResultsScreenProps {
  state: FlowState
  actions: AssistantActions
}

function fold(text: string): string {
  return text
    .normalize('NFD')
    .replace(/\p{Diacritic}/gu, '')
    .toLowerCase()
}

function fileLabel(file: GeneratedFile): string {
  return file.student_number === null
    ? file.student_name
    : `${String(file.student_number)}. ${file.student_name}`
}

export function ResultsScreen({ state, actions }: ResultsScreenProps) {
  const api = useApi()
  const results = state.results
  const [query, setQuery] = useState('')
  const [selectedPath, setSelectedPath] = useState<string | null>(null)
  const [actionError, setActionError] = useState<AppError | null>(null)

  const files = useMemo(() => results?.files ?? [], [results?.files])
  const visible = useMemo(() => {
    const needle = fold(query.trim())
    return needle === '' ? files : files.filter((file) => fold(file.student_name).includes(needle))
  }, [files, query])
  const selected = files.find((file) => file.path === selectedPath) ?? files[0] ?? null

  const navList = selected && visible.includes(selected) ? visible : files
  const index = selected ? navList.indexOf(selected) : -1
  const go = (delta: number) => {
    const target = navList[index + delta]
    if (target) setSelectedPath(target.path)
  }

  const selectedFilePath = selected?.path ?? null
  const load = useCallback(
    () => (selectedFilePath ? api.readPdf(selectedFilePath) : Promise.reject(new Error('no file'))),
    [api, selectedFilePath],
  )

  if (!results) return null

  const run = async (action: () => Promise<void>) => {
    setActionError(null)
    try {
      await action()
    } catch (error) {
      setActionError(toAppError(error))
    }
  }

  const openSelected = () => {
    if (selected) void run(() => api.openExternally(selected.path))
  }
  const fromHistory = results.origin === 'history'
  const failures = results.failures

  return (
    <ScreenFrame
      title={fromHistory ? t('results.historyTitle') : t('results.title')}
      windowPage={t('results.title')}
      subtitle={describeRun(results.subject, results.group)}
      back={<Button variant="outline" label={t('common.back')} onClick={actions.back} />}
      secondary={
        <>
          <Button
            variant="outline"
            icon={FolderOpen}
            label={t('results.actions.openFolder')}
            onClick={() => void run(() => api.revealInFolder(results.outputDir))}
          />
          <Button
            variant="outline"
            icon={ExternalLink}
            label={t('results.actions.openWith')}
            disabled={selected === null}
            onClick={openSelected}
          />
          <Button
            variant="outline"
            icon={Printer}
            label={t('results.actions.print')}
            disabled={selected === null}
            onClick={openSelected}
          />
        </>
      }
      primary={
        <Button
          label={t('results.actions.generateOthers')}
          onClick={actions.reset}
          data-primary-action=""
        />
      }
    >
      <div className="flex flex-col gap-4">
        {state.error ? <ErrorPanel error={state.error} /> : null}
        {actionError ? <ErrorPanel error={actionError} /> : null}

        {!fromHistory && !results.cancelled && files.length > 0 ? (
          <Notice variant="success">{t('results.done', { count: files.length })}</Notice>
        ) : null}
        {results.cancelled ? (
          <Notice variant="info">{t('generating.cancelled', { count: files.length })}</Notice>
        ) : null}

        {failures.length > 0 ? (
          <Notice
            variant="warning"
            title={t('generating.failures.title', { count: failures.length })}
          >
            <p>{t('generating.failures.description')}</p>
            <ul className="mt-2 list-disc pl-5">
              {failures.map((failure) => (
                <li key={`${String(failure.student_number)}-${failure.student_name}`}>
                  {t('generating.failures.student', {
                    name: failure.student_name,
                    reason: getErrorMessage(failure.code).title,
                  })}
                </li>
              ))}
            </ul>
            <TechnicalDetails
              text={failures
                .map((failure) => `${failure.student_name}: ${failure.code}: ${failure.message}`)
                .join('\n')}
            />
            {!fromHistory ? (
              <div className="mt-3">
                <Button
                  variant="outline"
                  icon={RefreshCw}
                  label={t('generating.failures.retry')}
                  onClick={() => void actions.retryFailures()}
                />
              </div>
            ) : null}
          </Notice>
        ) : null}

        {files.length === 0 ? (
          <EmptyState title={t('results.noneCreated')} description={t('results.noneCreatedHint')} />
        ) : (
          <div className="grid gap-5 lg:grid-cols-[18rem_1fr]">
            <section aria-labelledby="results-list-title" className="flex min-w-0 flex-col gap-3">
              <h2 id="results-list-title" className="text-2xl font-semibold">
                {t('results.listTitle')}
              </h2>
              <SearchField
                label={t('results.searchLabel')}
                value={query}
                onValueChange={setQuery}
              />
              {visible.length === 0 ? (
                <p className="text-fg-muted">{t('results.noMatches')}</p>
              ) : (
                <ul className="flex max-h-[26rem] flex-col gap-1 overflow-auto pr-1">
                  {visible.map((file) => {
                    const active = file === selected
                    return (
                      <li key={file.path}>
                        <button
                          type="button"
                          aria-current={active ? 'true' : undefined}
                          onClick={() => {
                            setSelectedPath(file.path)
                          }}
                          className={cn(
                            'min-h-11 w-full cursor-pointer rounded-md border-2 px-3 py-2 text-left break-words',
                            active
                              ? 'border-primary bg-surface-muted font-semibold'
                              : 'border-transparent hover:bg-surface-muted',
                          )}
                        >
                          {fileLabel(file)}
                        </button>
                      </li>
                    )
                  })}
                </ul>
              )}
            </section>

            <div className="flex min-w-0 flex-col gap-2">
              {selected ? (
                <PdfViewer
                  load={load}
                  documentKey={selected.path}
                  title={t('results.viewerFor', { name: selected.student_name })}
                  onPrevious={() => {
                    go(-1)
                  }}
                  onNext={() => {
                    go(1)
                  }}
                  hasPrevious={index > 0}
                  hasNext={index >= 0 && index < navList.length - 1}
                  className="h-[min(34rem,calc(100vh-22rem))] min-h-80"
                />
              ) : (
                <p className="text-fg-muted">{t('results.chooseOne')}</p>
              )}
              <p className="text-fg-muted">{t('results.actions.printHint')}</p>
            </div>
          </div>
        )}
      </div>
    </ScreenFrame>
  )
}
