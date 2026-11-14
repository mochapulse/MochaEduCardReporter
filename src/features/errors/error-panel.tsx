import { ChevronDown, ChevronUp, Copy } from 'lucide-react'
import { useId, useState } from 'react'
import { Button } from '@/components/ui/button'
import { Notice } from '@/components/ui/notice'
import { getErrorMessage } from '@/i18n/errors'
import { t } from '@/i18n/es'
import type { AppError } from '@/lib/api'

/** Technical text of an error: its message and detail, for "Ver detalles" and "Copiar". */
export function technicalText(error: AppError): string {
  return [`${error.code}: ${error.message}`, error.detail].filter(Boolean).join('\n')
}

/** "Ver detalles" disclosure with the technical text and a "Copiar" button. */
export function TechnicalDetails({ text }: { text: string }) {
  const [open, setOpen] = useState(false)
  const [copied, setCopied] = useState(false)
  const regionId = useId()

  const copy = async () => {
    try {
      await navigator.clipboard.writeText(text)
      setCopied(true)
    } catch {
      // Clipboard unavailable: the text stays selectable on screen.
    }
  }

  return (
    <div className="mt-3">
      <Button
        variant="ghost"
        icon={open ? ChevronUp : ChevronDown}
        label={open ? t('common.hideDetails') : t('common.seeDetails')}
        aria-expanded={open}
        aria-controls={regionId}
        onClick={() => {
          setOpen((value) => !value)
        }}
        className="-ml-5"
      />
      {open ? (
        <div id={regionId} className="mt-2 flex flex-col gap-3">
          <pre className="max-h-48 overflow-auto rounded-md border border-border bg-surface-muted p-3 text-sm whitespace-pre-wrap">
            {text}
          </pre>
          <div>
            <Button
              variant="outline"
              icon={Copy}
              label={copied ? t('common.copied') : t('common.copy')}
              onClick={() => {
                void copy()
              }}
            />
          </div>
        </div>
      ) : null}
    </div>
  )
}

export interface ErrorPanelProps {
  error: AppError
  /** Optional recovery action, e.g. a "Reintentar" button. */
  action?: React.ReactNode
  className?: string
}

/**
 * Shows an error the way the plan asks: what happened, what to do, and the
 * technical detail behind "Ver detalles". Unknown codes fall back to INTERNAL.
 */
export function ErrorPanel({ error, action, className }: ErrorPanelProps) {
  const message = getErrorMessage(error.code)
  return (
    <Notice variant="danger" title={message.title} className={className}>
      <p>
        <span className="font-semibold">{t('error.whatHappened')}: </span>
        {message.whatHappened}
      </p>
      <p className="mt-1">
        <span className="font-semibold">{t('error.whatToDo')}: </span>
        {message.whatToDo}
      </p>
      {action ? <div className="mt-3">{action}</div> : null}
      <TechnicalDetails text={technicalText(error)} />
    </Notice>
  )
}
