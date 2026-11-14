import { Search } from 'lucide-react'
import { useId } from 'react'
import { t } from '@/i18n/es'
import { cn } from '@/lib/utils'
import { Button } from './button'

export interface SearchFieldProps {
  label: string
  value: string
  onValueChange: (value: string) => void
  placeholder?: string
  className?: string
}

export function SearchField({
  label,
  value,
  onValueChange,
  placeholder,
  className,
}: SearchFieldProps) {
  const id = useId()
  return (
    <div className={className}>
      <label htmlFor={id} className="mb-2 block font-semibold">
        {label}
      </label>
      <div className="flex items-center gap-2">
        <div className="relative min-w-0 flex-1">
          <Search
            aria-hidden="true"
            className="pointer-events-none absolute top-1/2 left-3 size-5 -translate-y-1/2 text-fg-muted"
          />
          <input
            id={id}
            type="search"
            value={value}
            placeholder={placeholder}
            autoComplete="off"
            onChange={(event) => {
              onValueChange(event.target.value)
            }}
            onKeyDown={(event) => {
              if (event.key === 'Escape' && value !== '') {
                event.stopPropagation()
                onValueChange('')
              }
            }}
            className={cn(
              'min-h-11 w-full rounded-md border-2 border-fg-muted bg-surface py-2 pr-3 pl-10 text-fg',
              'placeholder:text-fg-muted [&::-webkit-search-cancel-button]:appearance-none',
            )}
          />
        </div>
        {value !== '' ? (
          <Button
            variant="outline"
            label={t('search.clear')}
            onClick={() => {
              onValueChange('')
            }}
          />
        ) : null}
      </div>
    </div>
  )
}
