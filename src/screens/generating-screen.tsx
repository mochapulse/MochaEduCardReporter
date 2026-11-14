import { useState } from 'react'
import { ScreenFrame } from '@/app/screen-frame'
import { Button } from '@/components/ui/button'
import { Card } from '@/components/ui/card'
import { ConfirmDialog } from '@/components/ui/dialog'
import { Notice } from '@/components/ui/notice'
import { ProgressBar } from '@/components/ui/progress-bar'
import { t } from '@/i18n/es'
import type { FlowState } from '@/features/assistant/state'
import type { AssistantActions } from '@/features/assistant/use-assistant'

export interface GeneratingScreenProps {
  state: FlowState
  actions: AssistantActions
}

export function GeneratingScreen({ state, actions }: GeneratingScreenProps) {
  const [confirming, setConfirming] = useState(false)
  const run = state.run
  const done = run?.done ?? 0
  const total = run?.total ?? 0
  const label =
    run?.currentName && done > 0
      ? t('progress.current', { done, total, name: run.currentName })
      : done > 0
        ? t('progress.finishing', { done, total })
        : t('generating.preparing')
  const failed = run?.failures.length ?? 0

  return (
    <ScreenFrame
      title={t('generating.title')}
      step={3}
      primary={
        <Button
          variant="outline"
          label={t('generating.cancel')}
          disabled={run?.cancelRequested === true}
          onClick={() => {
            setConfirming(true)
          }}
        />
      }
    >
      <Card className="flex flex-col gap-5">
        <p>{t('generating.inProgress')}</p>
        <ProgressBar value={done} max={total} label={label} />
        {run?.cancelRequested ? <Notice variant="info">{t('generating.cancelling')}</Notice> : null}
        {failed > 0 ? (
          <Notice variant="warning">{t('generating.failedSoFar', { count: failed })}</Notice>
        ) : null}
      </Card>
      <ConfirmDialog
        open={confirming}
        onOpenChange={setConfirming}
        title={t('generating.confirmCancel.title')}
        description={t('generating.confirmCancel.description')}
        confirmLabel={t('generating.confirmCancel.confirm')}
        cancelLabel={t('generating.confirmCancel.keep')}
        destructive
        onConfirm={() => void actions.cancelGeneration()}
      />
    </ScreenFrame>
  )
}
