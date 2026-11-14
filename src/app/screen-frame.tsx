import { useEffect, useRef, type ReactNode } from 'react'
import { Stepper } from '@/components/ui/stepper'
import { t } from '@/i18n/es'
import { useApi } from '@/lib/api'
import { cn } from '@/lib/utils'

export interface ScreenFrameProps {
  /** Heading of the screen; it receives focus when the screen appears. */
  title: string
  /** Name of the page in the window title (defaults to `title`). */
  windowPage?: string
  /** Current step (1-3) when the screen is part of the three-step assistant. */
  step?: number
  subtitle?: string
  /** Extra controls aligned with the heading (e.g. Ajustes / Ayuda). */
  headerActions?: ReactNode
  /** "Atrás" and other secondary actions, aligned to the left of the action bar. */
  back?: ReactNode
  /** Secondary actions next to the primary one. */
  secondary?: ReactNode
  /** The one primary action, always bottom-right. */
  primary?: ReactNode
  /** Explains why the primary action is disabled. */
  primaryNote?: string
  children: ReactNode
  className?: string
}

/**
 * Common page layout: heading (focused on arrival, so screen readers announce
 * the new screen), optional step indicator, content and a bottom action bar
 * whose primary action is always at the right.
 */
export function ScreenFrame({
  title,
  windowPage,
  step,
  subtitle,
  headerActions,
  back,
  secondary,
  primary,
  primaryNote,
  children,
  className,
}: ScreenFrameProps) {
  const api = useApi()
  const headingRef = useRef<HTMLHeadingElement>(null)
  const page = windowPage ?? title

  useEffect(() => {
    headingRef.current?.focus()
  }, [])

  useEffect(() => {
    void api.setWindowTitle(t('windowTitle.format', { page, app: t('app.name') }))
  }, [api, page])

  const hasBar = back || secondary || primary
  return (
    <div className="flex min-h-screen flex-col">
      <main className={cn('mx-auto w-full max-w-6xl flex-1 px-6 py-6', className)}>
        {step ? (
          <Stepper
            current={step}
            steps={[
              t('stepper.steps.review'),
              t('stepper.steps.save'),
              t('stepper.steps.generate'),
            ]}
            className="mb-5"
          />
        ) : null}
        <div className="mb-5 flex flex-wrap items-start justify-between gap-4">
          <div className="min-w-0">
            <h1
              ref={headingRef}
              tabIndex={-1}
              data-screen-heading=""
              className="text-3xl font-bold focus:outline-none"
            >
              {title}
            </h1>
            {subtitle ? <p className="mt-1 text-lg text-fg-muted">{subtitle}</p> : null}
          </div>
          {headerActions ? <div className="flex flex-wrap gap-2">{headerActions}</div> : null}
        </div>
        {children}
      </main>
      {hasBar ? (
        <footer className="sticky bottom-0 z-20 border-t border-border bg-background">
          <div className="mx-auto flex w-full max-w-6xl flex-wrap items-center justify-between gap-3 px-6 py-3">
            <div className="flex flex-wrap gap-3">{back}</div>
            <div className="flex flex-wrap items-center justify-end gap-3">
              {primaryNote ? <p className="text-fg-muted">{primaryNote}</p> : null}
              {secondary}
              {primary}
            </div>
          </div>
        </footer>
      ) : null}
    </div>
  )
}
