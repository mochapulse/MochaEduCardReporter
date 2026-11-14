import { useId } from 'react'
import { t } from '@/i18n/es'
import { cn } from '@/lib/utils'

export interface DateFieldProps {
  label: string
  /** `YYYY-MM-DD`, or an empty string while the person is typing. */
  value: string
  onValueChange: (value: string) => void
  /** The date written as DD/MM/YYYY, always shown so the format never depends on the system. */
  displayValue: string
  helpText?: string
  /** Error text; also marks the field as invalid. */
  errorText?: string
  className?: string
}

/**
 * Native date input (keyboard accessible, with the system date picker) plus a
 * line that always writes the chosen date as DD/MM/YYYY.
 */
export function DateField({
  label,
  value,
  onValueChange,
  displayValue,
  helpText,
  errorText,
  className,
}: DateFieldProps) {
  const id = useId()
  const describedBy = `${id}-desc`
  return (
    <div className={className}>
      <label htmlFor={id} className="mb-2 block font-semibold">
        {label}
      </label>
      <input
        id={id}
        type="date"
        value={value}
        required
        aria-invalid={errorText ? true : undefined}
        aria-describedby={describedBy}
        onChange={(event) => {
          onValueChange(event.target.value)
        }}
        className={cn(
          'min-h-11 rounded-md border-2 bg-surface px-3 py-2 text-fg',
          errorText ? 'border-danger' : 'border-fg-muted',
        )}
      />
      <p
        id={describedBy}
        className={cn('mt-2', errorText ? 'font-semibold text-danger' : 'text-fg-muted')}
      >
        {errorText ??
          [value === '' ? '' : t('save.datePrinted', { date: displayValue }), helpText]
            .filter(Boolean)
            .join(' ')}
      </p>
    </div>
  )
}
