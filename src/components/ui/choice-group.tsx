import { RadioGroup } from 'radix-ui'
import { useId, type ReactNode } from 'react'
import { cn } from '@/lib/utils'

export interface Choice<Value extends string> {
  value: Value
  label: string
  /** Optional explanation under the label. */
  description?: string
  icon?: ReactNode
}

export interface ChoiceGroupProps<Value extends string> {
  /** Visible group label (also its accessible name). */
  legend: string
  choices: readonly Choice<Value>[]
  value: Value | null
  onValueChange: (value: Value) => void
  /** Lay the options side by side (default) or one per row. */
  layout?: 'row' | 'column'
  className?: string
}

/**
 * One-of-many choice shown as large selectable cards (radio group).
 * Keyboard: Tab enters the group, arrow keys move between options.
 */
export function ChoiceGroup<Value extends string>({
  legend,
  choices,
  value,
  onValueChange,
  layout = 'row',
  className,
}: ChoiceGroupProps<Value>) {
  const legendId = useId()
  return (
    <div className={className}>
      <p id={legendId} className="mb-2 font-semibold">
        {legend}
      </p>
      <RadioGroup.Root
        aria-labelledby={legendId}
        value={value ?? ''}
        onValueChange={(next) => {
          const match = choices.find((choice) => choice.value === next)
          if (match) onValueChange(match.value)
        }}
        className={cn('flex gap-3', layout === 'row' ? 'flex-wrap' : 'flex-col')}
      >
        {choices.map((choice) => {
          const id = `${legendId}-${choice.value}`
          const selected = choice.value === value
          return (
            <label
              key={choice.value}
              htmlFor={id}
              data-selected={selected}
              className={cn(
                'flex min-h-11 min-w-0 flex-1 cursor-pointer items-start gap-3 rounded-md border-2 bg-surface p-4',
                selected
                  ? 'border-primary bg-surface-muted'
                  : 'border-fg-muted hover:bg-surface-muted',
                'has-[:focus-visible]:outline-[length:var(--focus-ring-width)] has-[:focus-visible]:outline-offset-2 has-[:focus-visible]:outline-focus-ring',
              )}
            >
              <RadioGroup.Item
                id={id}
                value={choice.value}
                className="mt-0.5 flex size-6 shrink-0 cursor-pointer items-center justify-center rounded-full border-2 border-fg-muted bg-surface data-[state=checked]:border-primary"
              >
                <RadioGroup.Indicator className="size-3 rounded-full bg-primary" />
              </RadioGroup.Item>
              <span className="min-w-0">
                <span className="flex items-center gap-2 font-semibold">
                  {choice.icon}
                  {choice.label}
                </span>
                {choice.description ? (
                  <span className="mt-1 block text-fg-muted">{choice.description}</span>
                ) : null}
              </span>
            </label>
          )
        })}
      </RadioGroup.Root>
    </div>
  )
}
