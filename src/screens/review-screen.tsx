import { Eye, FileSpreadsheet } from 'lucide-react'
import { useCallback, useEffect, useMemo, useState } from 'react'
import { ScreenFrame } from '@/app/screen-frame'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardTitle } from '@/components/ui/card'
import { DataTable, type DataTableColumn } from '@/components/ui/data-table'
import { EmptyState } from '@/components/ui/empty-state'
import { Modal } from '@/components/ui/modal'
import { Notice } from '@/components/ui/notice'
import { SearchField } from '@/components/ui/search-field'
import { PdfViewer } from '@/components/pdf-viewer'
import { t } from '@/i18n/es'
import { toAppError, useApi, type AppError, type GradebookView } from '@/lib/api'
import { usableSheets, type FlowState } from '@/features/assistant/state'
import type { AssistantActions } from '@/features/assistant/use-assistant'
import { ErrorPanel, TechnicalDetails } from '@/features/errors/error-panel'
import { translateWarning } from '@/features/errors/warnings'
import { valueOrDash } from '@/features/format'
import {
  filterRows,
  firstPreviewable,
  mergeVisibleSelection,
  toRows,
  type StudentRow,
} from '@/features/review/students'

export interface ReviewScreenProps {
  state: FlowState
  actions: AssistantActions
}

export function ReviewScreen({ state, actions }: ReviewScreenProps) {
  const back = <Button variant="outline" label={t('common.back')} onClick={actions.back} />
  const inspecting = state.busy?.kind === 'inspecting'

  if (state.reviewStep === 'sheet' || !state.gradebook) {
    const sheets = state.source ? usableSheets(state.source.sheets) : []
    return (
      <ScreenFrame
        title={t('review.chooseSheet')}
        windowPage={t('review.title')}
        step={1}
        back={back}
      >
        {state.error ? <ErrorPanel error={state.error} className="mb-5" /> : null}
        {inspecting ? (
          <Notice variant="info" className="mb-5">
            {t('review.opening')}
          </Notice>
        ) : null}
        <ul className="grid gap-4 md:grid-cols-2">
          {sheets.map((sheet) => (
            <li key={sheet.name}>
              <Card className="flex h-full flex-col justify-between gap-4">
                <div>
                  <CardTitle level={2} className="flex items-center gap-2 break-words">
                    <FileSpreadsheet aria-hidden="true" className="size-6 shrink-0 text-primary" />
                    {sheet.name}
                  </CardTitle>
                  {sheet.student_count !== null ? (
                    <CardContent className="text-lg">
                      {t('review.sheetStudents', { count: sheet.student_count })}
                    </CardContent>
                  ) : null}
                </div>
                <div>
                  <Button
                    variant="outline"
                    label={t('review.useSheet')}
                    aria-label={`${t('review.useSheet')}: ${sheet.name}`}
                    disabled={inspecting}
                    onClick={() => void actions.chooseSheet(sheet.name)}
                  />
                </div>
              </Card>
            </li>
          ))}
        </ul>
      </ScreenFrame>
    )
  }

  return <StudentsStep gradebook={state.gradebook} state={state} actions={actions} back={back} />
}

function StudentsStep({
  gradebook,
  state,
  actions,
  back,
}: ReviewScreenProps & { gradebook: GradebookView; back: React.ReactNode }) {
  const [query, setQuery] = useState('')
  const [previewOpen, setPreviewOpen] = useState(false)

  const rows = useMemo(() => toRows(gradebook.students), [gradebook.students])
  const visible = useMemo(() => filterRows(rows, query), [rows, query])
  const selectedCount = rows.filter((row) => state.selected.has(row.key)).length
  const warnings = useMemo(() => gradebook.warnings.map(translateWarning), [gradebook.warnings])
  const previewRow = firstPreviewable(rows, state.selected)

  const columns = useMemo<readonly DataTableColumn<StudentRow>[]>(
    () => [
      {
        id: 'number',
        header: t('review.students.number'),
        cell: (row) => row.student.number ?? t('common.emptyValue'),
        align: 'right',
      },
      { id: 'name', header: t('review.students.name'), cell: (row) => row.student.name },
      { id: 'code', header: t('review.students.code'), cell: (row) => row.student.code },
      {
        id: 'absences',
        header: t('review.students.absences'),
        cell: (row) => row.student.absences,
        align: 'right',
      },
      {
        id: 'grade',
        header: t('review.students.finalGrade'),
        cell: (row) => row.student.definitive_grade,
        align: 'right',
      },
    ],
    [],
  )

  const summary: [string, string][] = [
    ...(gradebook.sheet
      ? ([[t('review.summary.sheet'), gradebook.sheet]] as [string, string][])
      : []),
    [t('review.summary.subject'), gradebook.metadata.subject],
    [t('review.summary.group'), gradebook.metadata.group],
    [t('review.summary.teacher'), gradebook.metadata.professor],
    [t('review.summary.period'), gradebook.metadata.period],
  ]

  const canContinue = selectedCount > 0
  return (
    <ScreenFrame
      title={t('review.title')}
      subtitle={gradebook.source_file_name}
      step={1}
      back={back}
      primaryNote={canContinue ? undefined : t('review.nothingSelected')}
      primary={
        <Button
          label={t('common.continue')}
          disabled={!canContinue}
          onClick={actions.continueToSave}
          data-primary-action=""
        />
      }
    >
      <div className="flex flex-col gap-5">
        {state.error ? <ErrorPanel error={state.error} /> : null}

        <Card className="p-5">
          <CardTitle level={2} className="sr-only">
            {t('review.summary.title')}
          </CardTitle>
          <dl className="grid gap-x-6 gap-y-3 sm:grid-cols-2 lg:grid-cols-5">
            {summary.map(([label, value]) => (
              <div key={label} className="min-w-0">
                <dt className="text-fg-muted">{label}</dt>
                <dd className="text-lg font-semibold break-words">{valueOrDash(value)}</dd>
              </div>
            ))}
          </dl>
        </Card>

        {warnings.length > 0 ? (
          <Notice variant="warning" title={t('review.warningsTitle')}>
            <ul className="list-disc pl-5">
              {warnings.map((warning) => (
                <li key={warning.original}>{warning.text}</li>
              ))}
            </ul>
            <TechnicalDetails text={warnings.map((warning) => warning.original).join('\n')} />
          </Notice>
        ) : null}

        <section aria-labelledby="students-title" className="flex flex-col gap-3">
          <h2 id="students-title" className="text-2xl font-semibold">
            {t('review.students.title')}
          </h2>
          <div className="flex flex-wrap items-end gap-3">
            <SearchField
              className="min-w-64 flex-1"
              label={t('review.students.searchLabel')}
              placeholder={t('review.students.searchPlaceholder')}
              value={query}
              onValueChange={setQuery}
            />
            <Button
              variant="outline"
              label={t('table.selectAll')}
              onClick={() => {
                actions.setSelection(new Set(rows.map((row) => row.key)))
              }}
            />
            <Button
              variant="outline"
              label={t('table.selectNone')}
              onClick={() => {
                actions.setSelection(new Set())
              }}
            />
            <Button
              variant="outline"
              icon={Eye}
              label={t('review.preview')}
              disabled={previewRow === null}
              onClick={() => {
                setPreviewOpen(true)
              }}
            />
          </div>
          <p className="text-fg-muted">{t('review.previewHint')}</p>
          <DataTable
            caption={t('review.students.caption')}
            columns={columns}
            rows={visible}
            getRowId={(row) => row.key}
            getRowLabel={(row) => t('table.selectRow', { name: row.student.name })}
            selectedIds={state.selected}
            onSelectedIdsChange={(next) => {
              actions.setSelection(mergeVisibleSelection(rows, visible, state.selected, next))
            }}
            maxHeight="min(22rem, calc(100vh - 26rem))"
            showSummary={false}
            emptyState={
              <EmptyState
                title={
                  query.trim() === ''
                    ? t('empty.noStudents')
                    : t('search.noResults', { query: query.trim() })
                }
                description={query.trim() === '' ? undefined : t('search.noResultsHint')}
              />
            }
          />
          <p aria-live="polite" className="text-fg-muted">
            {t('review.students.selected', { count: selectedCount, total: rows.length })}
          </p>
        </section>
      </div>

      {previewOpen && previewRow ? (
        <PreviewModal
          gradebook={gradebook}
          row={previewRow}
          dateIso={state.save.date}
          onClose={() => {
            setPreviewOpen(false)
          }}
        />
      ) : null}
    </ScreenFrame>
  )
}

function PreviewModal({
  gradebook,
  row,
  dateIso,
  onClose,
}: {
  gradebook: GradebookView
  row: StudentRow
  dateIso: string
  onClose: () => void
}) {
  const api = useApi()
  const number = row.student.number
  const [outcome, setOutcome] = useState<
    { key: string; path: string } | { key: string; error: AppError } | null
  >(null)
  const key = `${gradebook.id}:${row.key}`

  // The example is created once per opening; the viewer then loads its bytes.
  useEffect(() => {
    if (number === null) return
    let alive = true
    api.previewStudent(gradebook.id, number, dateIso).then(
      (path) => {
        if (alive) setOutcome({ key, path })
      },
      (error: unknown) => {
        if (alive) setOutcome({ key, error: toAppError(error) })
      },
    )
    return () => {
      alive = false
    }
  }, [api, gradebook.id, number, dateIso, key])

  const current = outcome?.key === key ? outcome : null
  const path = current && 'path' in current ? current.path : null
  const load = useCallback(
    () => (path ? api.readPdf(path) : Promise.reject(new Error('no path'))),
    [api, path],
  )

  return (
    <Modal
      open
      onOpenChange={(open) => {
        if (!open) onClose()
      }}
      title={t('review.previewTitle')}
      description={t('review.previewFor', { name: row.student.name })}
    >
      {current === null ? <Notice variant="info">{t('review.previewPreparing')}</Notice> : null}
      {current && 'error' in current ? <ErrorPanel error={current.error} /> : null}
      {path ? (
        <PdfViewer
          load={load}
          documentKey={path}
          title={t('results.viewerFor', { name: row.student.name })}
          className="h-full"
        />
      ) : null}
    </Modal>
  )
}
