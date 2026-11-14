import { Checkbox as CheckboxPrimitive } from 'radix-ui'
import { Check, Minus } from 'lucide-react'
import { useId } from 'react'
import { cn } from '@/lib/utils'

export interface CheckboxProps {
  /** Accessible name. Shown next to the box unless `hideLabel` is set. */
  label: string
  /** Keep the label for assistive technology only (e.g. one box per table row). */
  hideLabel?: boolean
  checked: boolean | 'indeterminate'
  onCheckedChange: (checked: boolean) => void
  disabled?: boolean
  className?: string
}

export function Checkbox({
  label,
  hideLabel = false,
  checked,
  onCheckedChange,
  disabled,
  className,
}: CheckboxProps) {
  const id = useId()
  return (
    <div className={cn('inline-flex min-h-11 min-w-11 items-center gap-3', className)}>
      <CheckboxPrimitive.Root
        id={id}
        checked={checked}
        disabled={disabled}
        onCheckedChange={(next) => {
          onCheckedChange(next === true)
        }}
        className={cn(
          'flex size-7 shrink-0 cursor-pointer items-center justify-center rounded-sm border-2 border-fg-muted bg-surface text-on-primary',
          'data-[state=checked]:border-primary data-[state=checked]:bg-primary',
          'data-[state=indeterminate]:border-primary data-[state=indeterminate]:bg-primary',
          'disabled:cursor-not-allowed disabled:opacity-50',
          // The invisible padding extends the hit area to 44 px.
          'relative before:absolute before:-inset-2 before:content-[""]',
        )}
      >
        <CheckboxPrimitive.Indicator>
          {checked === 'indeterminate' ? (
            <Minus aria-hidden="true" className="size-5" strokeWidth={3} />
          ) : (
            <Check aria-hidden="true" className="size-5" strokeWidth={3} />
          )}
        </CheckboxPrimitive.Indicator>
      </CheckboxPrimitive.Root>
      <label
        htmlFor={id}
        className={cn('cursor-pointer text-base', hideLabel && 'sr-only', disabled && 'opacity-50')}
      >
        {label}
      </label>
    </div>
  )
}
