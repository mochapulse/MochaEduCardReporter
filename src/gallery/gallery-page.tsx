import { FileText, FolderOpen, Trash2 } from 'lucide-react'
import { useState, type ReactNode } from 'react'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardTitle } from '@/components/ui/card'
import { Checkbox } from '@/components/ui/checkbox'
import { DataTable, type DataTableColumn } from '@/components/ui/data-table'
import { ConfirmDialog } from '@/components/ui/dialog'
import { DropZone } from '@/components/ui/drop-zone'
import { EmptyState } from '@/components/ui/empty-state'
import { Notice } from '@/components/ui/notice'
import { PathField } from '@/components/ui/path-field'
import { ProgressBar } from '@/components/ui/progress-bar'
import { SearchField } from '@/components/ui/search-field'
import { Stepper } from '@/components/ui/stepper'
import { Tooltip } from '@/components/ui/tooltip'
import { errorCodes, errorMessages } from '@/i18n/errors'
import { es, t } from '@/i18n/es'
import { students, type Student } from './sample-data'
import { textSizes, useTextSize, type TextSize } from '@/lib/text-size'
import { themePreferences, useTheme, type ThemePreference } from '@/lib/theme'

const columns: readonly DataTableColumn<Student>[] = [
  { id: 'number', header: es.review.students.number, cell: (row) => row.number, align: 'right' },
  { id: 'name', header: es.review.students.name, cell: (row) => row.name },
  { id: 'code', header: es.review.students.code, cell: (row) => row.code },
  {
    id: 'absences',
    header: es.review.students.absences,
    cell: (row) => row.absences,
    align: 'right',
  },
  {
    id: 'grade',
    header: es.review.students.finalGrade,
    cell: (row) => row.finalGrade,
    align: 'right',
  },
]

function Section({ title, children }: { title: string; children: ReactNode }) {
  return (
    <section className="mt-10">
      <h2 className="mb-4 border-b border-border pb-2 text-2xl font-semibold">{title}</h2>
      <div className="flex flex-col gap-6">{children}</div>
    </section>
  )
}

function Demo({ label, children }: { label: string; children: ReactNode }) {
  return (
    <div>
      <h3 className="mb-2 text-sm font-semibold tracking-wide text-fg-muted uppercase">{label}</h3>
      {children}
    </div>
  )
}

const themeLabels: Record<ThemePreference, string> = {
  claro: es.settings.theme.light,
  oscuro: es.settings.theme.dark,
  sistema: es.settings.theme.system,
}

const sizeLabels: Record<TextSize, string> = {
  normal: es.settings.textSize.normal,
  grande: es.settings.textSize.large,
  'muy-grande': es.settings.textSize.extraLarge,
}

/** Developer/QA page: every component in every state, plus theme and text-size controls. */
export function GalleryPage() {
  const { preference, setPreference } = useTheme()
  const { size, setSize } = useTextSize()
  const g = es.gallery
  const sample = g.sample

  const [selected, setSelected] = useState<ReadonlySet<string>>(new Set(['2', '3']))
  const [query, setQuery] = useState('')
  const [checked, setChecked] = useState(true)
  const [dialogOpen, setDialogOpen] = useState(false)
  const [dialogResult, setDialogResult] = useState<string | null>(null)
  const [event, setEvent] = useState<string | null>(null)

  return (
    <main className="mx-auto max-w-5xl px-6 py-8">
      <h1 className="text-3xl font-bold">{g.title}</h1>
      <p className="mt-2 text-fg-muted">{g.intro}</p>

      <Card className="mt-6">
        <CardTitle level={2}>{g.appearance}</CardTitle>
        <div className="mt-4 flex flex-wrap gap-x-10 gap-y-4">
          <fieldset>
            <legend className="mb-2 font-semibold">{g.theme}</legend>
            <div className="flex flex-wrap gap-2">
              {themePreferences.map((option) => (
                <Button
                  key={option}
                  label={themeLabels[option]}
                  variant={preference === option ? 'primary' : 'outline'}
                  aria-pressed={preference === option}
                  onClick={() => {
                    setPreference(option)
                  }}
                />
              ))}
            </div>
          </fieldset>
          <fieldset>
            <legend className="mb-2 font-semibold">{g.textSize}</legend>
            <div className="flex flex-wrap gap-2">
              {textSizes.map((option) => (
                <Button
                  key={option}
                  label={sizeLabels[option]}
                  variant={size === option ? 'primary' : 'outline'}
                  aria-pressed={size === option}
                  onClick={() => {
                    setSize(option)
                  }}
                />
              ))}
            </div>
          </fieldset>
        </div>
      </Card>

      <Section title={g.sections.buttons}>
        <Demo label={g.states.normal}>
          <div className="flex flex-wrap gap-3">
            <Button label={sample.primary} />
            <Button variant="outline" label={sample.secondary} />
            <Button variant="ghost" label={sample.ghost} />
            <Button variant="danger" label={sample.danger} />
          </div>
        </Demo>
        <Demo label={g.states.withIcon}>
          <div className="flex flex-wrap gap-3">
            <Button icon={FileText} label={t('save.generate', { count: 27 })} />
            <Button variant="outline" icon={FolderOpen} label={es.results.actions.openFolder} />
            <Button variant="ghost" icon={FileText} label={es.review.preview} />
            <Button variant="danger" icon={Trash2} label={sample.danger} />
          </div>
        </Demo>
        <Demo label={g.states.disabled}>
          <div className="flex flex-wrap items-center gap-3">
            <Button disabled label={t('save.generate', { count: 27 })} />
            <Button disabled variant="outline" label={sample.secondary} />
            <p>{es.save.disabledReason}</p>
          </div>
        </Demo>
      </Section>

      <Section title={g.sections.cards}>
        <Card>
          <CardTitle>{sample.cardTitle}</CardTitle>
          <CardContent>{sample.cardBody}</CardContent>
        </Card>
      </Section>

      <Section title={g.sections.stepper}>
        {[1, 2, 3].map((current) => (
          <Demo key={current} label={t('gallery.states.step', { current })}>
            <Stepper
              label={`${es.stepper.label} — ${t('gallery.states.step', { current })}`}
              current={current}
              steps={[es.stepper.steps.review, es.stepper.steps.save, es.stepper.steps.generate]}
            />
          </Demo>
        ))}
      </Section>

      <Section title={g.sections.table}>
        <DataTable
          caption={es.review.students.title}
          columns={columns}
          rows={students}
          getRowId={(row) => row.id}
          getRowLabel={(row) => t('table.selectRow', { name: row.name })}
          selectedIds={selected}
          onSelectedIdsChange={setSelected}
          emptyState={<EmptyState title={es.empty.noStudents} />}
          maxHeight="14rem"
        />
        <Demo label={g.states.empty}>
          <DataTable
            caption={es.review.students.title}
            columns={columns}
            rows={[]}
            getRowId={(row) => row.id}
            getRowLabel={(row) => row.name}
            selectedIds={new Set()}
            onSelectedIdsChange={() => undefined}
            emptyState={
              <EmptyState
                icon={FileText}
                title={sample.emptyTitle}
                description={sample.emptyDescription}
              />
            }
          />
        </Demo>
      </Section>

      <Section title={g.sections.notices}>
        <Notice variant="info">{sample.noticeInfo}</Notice>
        <Notice variant="success">{sample.noticeSuccess}</Notice>
        <Notice variant="warning">{sample.noticeWarning}</Notice>
        <Notice variant="danger" title={sample.noticeTitle}>
          {sample.noticeDanger}
        </Notice>
      </Section>

      <Section title={g.sections.dialog}>
        <div className="flex flex-wrap items-center gap-3">
          <Button
            variant="danger"
            label={sample.openDialog}
            onClick={() => {
              setDialogOpen(true)
            }}
          />
          {dialogResult ? (
            <p>{t('gallery.sample.dialogResult', { result: dialogResult })}</p>
          ) : null}
        </div>
        <ConfirmDialog
          destructive
          open={dialogOpen}
          onOpenChange={setDialogOpen}
          title={sample.dialogTitle}
          description={sample.dialogDescription}
          confirmLabel={sample.dialogConfirm}
          cancelLabel={sample.dialogCancel}
          onConfirm={() => {
            setDialogResult(es.common.yes)
          }}
        />
      </Section>

      <Section title={g.sections.progress}>
        <ProgressBar
          value={12}
          max={27}
          label={t('progress.current', { done: 12, total: 27, name: sample.progressName })}
        />
        <ProgressBar value={27} max={27} label={t('progress.finishing', { done: 27, total: 27 })} />
      </Section>

      <Section title={g.sections.empty}>
        <EmptyState
          icon={FileText}
          title={sample.emptyTitle}
          description={sample.emptyDescription}
          action={<Button variant="outline" label={sample.emptyAction} />}
        />
      </Section>

      <Section title={g.sections.pathField}>
        <PathField
          label={es.save.savingIn}
          path={sample.folder}
          onChange={() => {
            setEvent(sample.changeFolderClicked)
          }}
        />
        {event ? <p role="status">{event}</p> : null}
      </Section>

      <Section title={g.sections.search}>
        <SearchField
          label={sample.searchLabel}
          placeholder={sample.searchPlaceholder}
          value={query}
          onValueChange={setQuery}
        />
      </Section>

      <Section title={g.sections.checkbox}>
        <div className="flex flex-wrap gap-6">
          <Checkbox label={sample.checkboxLabel} checked={checked} onCheckedChange={setChecked} />
          <Checkbox label={g.states.unchecked} checked={false} onCheckedChange={() => undefined} />
          <Checkbox
            label={g.states.indeterminate}
            checked="indeterminate"
            onCheckedChange={() => undefined}
          />
          <Checkbox
            disabled
            label={g.states.disabled}
            checked={false}
            onCheckedChange={() => undefined}
          />
        </div>
      </Section>

      <Section title={g.sections.dropZone}>
        <Demo label={g.states.normal}>
          <DropZone
            onChooseFile={() => {
              setEvent(sample.chosenFile)
            }}
            onDropFiles={(accepted) => {
              setEvent(accepted.length > 0 ? sample.droppedFile : sample.rejectedFile)
            }}
          />
        </Demo>
        <Demo label={g.states.dragOver}>
          <DropZone forceDragOver onChooseFile={() => undefined} />
        </Demo>
      </Section>

      <Section title={g.sections.tooltip}>
        <div>
          <Tooltip content={sample.tooltipContent}>
            <Button variant="ghost" label={sample.tooltipTrigger} />
          </Tooltip>
        </div>
      </Section>

      <Section title={g.sections.errors}>
        {errorCodes.map((code) => {
          const message = errorMessages[code]
          return (
            <Notice
              key={code}
              variant={code === 'CANCELLED' ? 'info' : 'danger'}
              title={message.title}
            >
              <p>
                <strong>{es.error.whatHappened}:</strong> {message.whatHappened}
              </p>
              <p>
                <strong>{es.error.whatToDo}:</strong> {message.whatToDo}
              </p>
            </Notice>
          )
        })}
      </Section>
    </main>
  )
}
