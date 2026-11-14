import { Folder } from 'lucide-react'
import { useId } from 'react'
import { t } from '@/i18n/es'
import { cn } from '@/lib/utils'
import { Button } from './button'
import { Tooltip } from './tooltip'

/** Splits a path so the last two segments stay visible while the start is truncated. */
export function splitPathForDisplay(path: string): { head: string; tail: string } {
  const separator = path.includes('\\') && !path.includes('/') ? '\\' : '/'
  const parts = path.split(separator)
  if (parts.length <= 2) return { head: '', tail: path }
  const tail = parts.slice(-2).join(separator)
  return { head: path.slice(0, path.length - tail.length), tail }
}

export interface PathFieldProps {
  /** Label shown above the path, e.g. "Los boletines se guardarán en:". */
  label: string
  path: string
  onChange: () => void
  changeLabel?: string
  className?: string
}

/**
 * Shows a folder path readably: the middle is truncated (start ellipsized, last
 * two segments always visible). The full path is available on hover and focus,
 * and is what screen readers read.
 */
export function PathField({
  label,
  path,
  onChange,
  changeLabel = t('path.change'),
  className,
}: PathFieldProps) {
  const labelId = useId()
  const { head, tail } = splitPathForDisplay(path)
  return (
    <div className={className}>
      <p id={labelId} className="mb-2 font-semibold">
        {label}
      </p>
      <div className="flex flex-wrap items-center gap-3">
        <Tooltip content={t('path.fullPathHint', { path })}>
          {/* Focusable on purpose: keyboard users reveal the full path through the tooltip. */}
          <div
            role="group"
            tabIndex={0} // eslint-disable-line jsx-a11y/no-noninteractive-tabindex
            aria-labelledby={labelId}
            className="flex min-h-11 min-w-0 flex-1 items-center gap-2 rounded-md border border-fg-muted bg-surface px-3 py-2"
          >
            <Folder aria-hidden="true" className="size-5 shrink-0 text-fg-muted" />
            <span className="sr-only">{path}</span>
            <span aria-hidden="true" className="flex min-w-0 flex-1 font-mono text-sm">
              {head ? <span className="truncate">{head}</span> : null}
              <span className={cn('shrink-0', !head && 'truncate')}>{tail}</span>
            </span>
          </div>
        </Tooltip>
        <Button variant="outline" label={changeLabel} onClick={onChange} />
      </div>
    </div>
  )
}
