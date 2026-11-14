import { FilePlus2, FolderOpen, ScanSearch } from 'lucide-react'
import { ScreenFrame } from '@/app/screen-frame'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardTitle } from '@/components/ui/card'
import { t } from '@/i18n/es'
import type { AssistantActions } from '@/features/assistant/use-assistant'

export function HelpScreen({ actions }: { actions: AssistantActions }) {
  const steps = [
    { icon: FilePlus2, text: t('helpPage.step1') },
    { icon: FolderOpen, text: t('helpPage.step2') },
    { icon: ScanSearch, text: t('helpPage.step3') },
  ]
  return (
    <ScreenFrame
      title={t('helpPage.title')}
      subtitle={t('helpPage.intro')}
      back={<Button variant="outline" label={t('common.back')} onClick={actions.back} />}
    >
      <div className="flex flex-col gap-5">
        <Card>
          <CardTitle level={2}>{t('helpPage.stepsTitle')}</CardTitle>
          <ol className="mt-4 flex flex-col gap-4">
            {steps.map(({ icon: Icon, text }, index) => (
              <li key={text} className="flex items-center gap-4">
                <span
                  aria-hidden="true"
                  className="flex size-12 shrink-0 items-center justify-center rounded-full bg-surface-muted text-primary"
                >
                  <Icon className="size-6" />
                </span>
                <p className="text-lg">
                  <span className="font-semibold">{index + 1}. </span>
                  {text}
                </p>
              </li>
            ))}
          </ol>
        </Card>
        <Card>
          <CardTitle level={2}>{t('helpPage.formatsTitle')}</CardTitle>
          <CardContent className="text-lg text-fg">{t('helpPage.formats')}</CardContent>
        </Card>
        <Card>
          <CardTitle level={2}>{t('helpPage.notRecognizedTitle')}</CardTitle>
          <CardContent className="text-lg text-fg">{t('helpPage.notRecognized')}</CardContent>
        </Card>
      </div>
    </ScreenFrame>
  )
}
