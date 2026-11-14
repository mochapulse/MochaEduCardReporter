import { Check } from 'lucide-react'
import { t } from '@/i18n/es'
import { cn } from '@/lib/utils'

export interface StepperProps {
  /** Names of the steps, in order. */
  steps: readonly string[]
  /** Current step, 1-based. */
  current: number
  /** Accessible name of the navigation landmark; only override when several steppers share a page. */
  label?: string
  className?: string
}

type StepState = 'done' | 'current' | 'upcoming'

export function Stepper({ steps, current, label = t('stepper.label'), className }: StepperProps) {
  return (
    <nav aria-label={label} className={className}>
      <p className="mb-3 text-lg font-semibold">
        {t('stepper.progress', { current, total: steps.length })}
      </p>
      <ol className="flex flex-wrap gap-x-6 gap-y-2">
        {steps.map((name, index) => {
          const number = index + 1
          const state: StepState =
            number < current ? 'done' : number === current ? 'current' : 'upcoming'
          return (
            <li
              key={name}
              aria-current={state === 'current' ? 'step' : undefined}
              data-state={state}
              className={cn(
                'flex items-center gap-2',
                state === 'current' ? 'font-semibold text-fg' : 'text-fg-muted',
              )}
            >
              <span
                aria-hidden="true"
                className={cn(
                  'flex size-8 shrink-0 items-center justify-center rounded-full border-2 text-sm font-semibold',
                  state === 'done' && 'border-primary bg-primary text-on-primary',
                  state === 'current' && 'border-primary bg-surface text-primary',
                  state === 'upcoming' && 'border-fg-muted bg-surface text-fg-muted',
                )}
              >
                {state === 'done' ? <Check className="size-4" /> : number}
              </span>
              <span
                className={cn(state === 'current' && 'underline decoration-2 underline-offset-4')}
              >
                {name}
                <span className="sr-only"> ({t(`stepper.state.${state}`)})</span>
              </span>
            </li>
          )
        })}
      </ol>
    </nav>
  )
}
