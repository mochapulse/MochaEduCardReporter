import { ScreenFrame } from '@/app/screen-frame'
import { Button } from '@/components/ui/button'
import { Card } from '@/components/ui/card'
import { Checkbox } from '@/components/ui/checkbox'
import { ChoiceGroup } from '@/components/ui/choice-group'
import { DateField } from '@/components/ui/date-field'
import { Notice } from '@/components/ui/notice'
import { PathField } from '@/components/ui/path-field'
import { t } from '@/i18n/es'
import { formatDisplayDate, isValidIsoDate } from '@/features/assistant/dates'
import { baseName } from '@/features/assistant/paths'
import {
  generateBlockReason,
  outputFolder,
  studentKey,
  type BlockReason,
  type ExistingChoice,
  type FlowState,
} from '@/features/assistant/state'
import type { AssistantActions } from '@/features/assistant/use-assistant'
import { ErrorPanel } from '@/features/errors/error-panel'

const reasonKey = {
  noFolder: 'save.disabledReason',
  badDate: 'save.dateInvalid',
  checking: 'save.checking',
  chooseExisting: 'save.chooseExisting',
  notWritable: 'save.notWritable',
} as const satisfies Record<BlockReason, Parameters<typeof t>[0]>

export interface SaveScreenProps {
  state: FlowState
  actions: AssistantActions
}

export function SaveScreen({ state, actions }: SaveScreenProps) {
  const gradebook = state.gradebook
  const count = gradebook
    ? gradebook.students.filter((student, index) => state.selected.has(studentKey(student, index)))
        .length
    : 0
  const folder = outputFolder(state)
  const reason = generateBlockReason(state)
  const status = state.save.status
  const hasExisting = status !== null && status.pdf_count > 0 && state.save.statusFor !== null
  const dateValid = isValidIsoDate(state.save.date)

  return (
    <ScreenFrame
      title={t('save.title')}
      step={2}
      back={<Button variant="outline" label={t('common.back')} onClick={actions.back} />}
      primaryNote={reason ? t(reasonKey[reason]) : undefined}
      primary={
        <Button
          label={t('save.generate', { count })}
          disabled={reason !== null}
          onClick={() => void actions.generate()}
          data-primary-action=""
        />
      }
    >
      <div className="flex flex-col gap-5">
        {state.error ? <ErrorPanel error={state.error} /> : null}

        <Card className="flex flex-col gap-5">
          <PathField
            label={t('save.savingIn')}
            path={folder ?? ''}
            changeLabel={t('save.changeFolder')}
            onChange={() => void actions.changeFolder()}
          />
          <Checkbox
            label={t('save.rememberBase')}
            checked={state.save.remember}
            onCheckedChange={actions.setRemember}
          />
        </Card>

        {hasExisting ? (
          <div className="flex flex-col gap-4">
            <Notice variant="warning" title={t('save.folderExists.title')}>
              <p>{t('save.folderExists.found', { count: status.pdf_count })}</p>
              <p className="mt-1">{t('save.folderExists.description')}</p>
            </Notice>
            <ChoiceGroup<ExistingChoice>
              legend={t('save.folderExists.choose')}
              layout="row"
              value={state.save.existing}
              onValueChange={actions.setExistingChoice}
              choices={[
                {
                  value: 'replace',
                  label: t('save.folderExists.replace'),
                  description: t('save.folderExists.replaceHint'),
                },
                {
                  value: 'new',
                  label: t('save.folderExists.newFolder'),
                  description: t('save.folderExists.newFolderHint', {
                    name: baseName(status.alternative_path),
                  }),
                },
              ]}
            />
          </div>
        ) : null}

        <Card>
          <DateField
            label={t('save.dateLabel')}
            value={state.save.date}
            onValueChange={actions.setDate}
            displayValue={formatDisplayDate(state.save.date)}
            helpText={t('save.dateHelp')}
            errorText={dateValid ? undefined : t('save.dateInvalid')}
          />
        </Card>

        {folder ? (
          <p className="text-lg" aria-live="polite">
            {t('save.summary', { count, path: folder })}
          </p>
        ) : null}
      </div>
    </ScreenFrame>
  )
}
