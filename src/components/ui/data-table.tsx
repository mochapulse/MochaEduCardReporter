import { useRef, type KeyboardEvent, type ReactNode } from 'react'
import { t } from '@/i18n/es'
import { cn } from '@/lib/utils'
import { Checkbox } from './checkbox'

export interface DataTableColumn<Row> {
  id: string
  header: string
  cell: (row: Row) => ReactNode
  align?: 'left' | 'right' | 'center'
}

export interface DataTableProps<Row> {
  /** Accessible name of the table. */
  caption: string
  columns: readonly DataTableColumn<Row>[]
  rows: readonly Row[]
  getRowId: (row: Row) => string
  /** Accessible name of the checkbox of one row, e.g. "Seleccionar a MARTÍNEZ, Ana". */
  getRowLabel: (row: Row) => string
  selectedIds: ReadonlySet<string>
  onSelectedIdsChange: (next: ReadonlySet<string>) => void
  /** Shown instead of the table when there are no rows. */
  emptyState: ReactNode
  /** CSS max height of the scroll area (the header stays visible). */
  maxHeight?: string
  /** Show the "N seleccionados de M" line under the table (default true). */
  showSummary?: boolean
  className?: string
}

const alignClass = { left: 'text-left', right: 'text-right', center: 'text-center' } as const

/**
 * Table with sticky header, row checkboxes and a select-all checkbox with an
 * indeterminate state. Keyboard: Tab reaches each checkbox, Space toggles it,
 * Arrow Up/Down move between row checkboxes.
 */
export function DataTable<Row>({
  caption,
  columns,
  rows,
  getRowId,
  getRowLabel,
  selectedIds,
  onSelectedIdsChange,
  emptyState,
  maxHeight = '24rem',
  showSummary = true,
  className,
}: DataTableProps<Row>) {
  const regionRef = useRef<HTMLDivElement>(null)

  if (rows.length === 0) return <div className={className}>{emptyState}</div>

  const ids = rows.map(getRowId)
  const selectedCount = ids.filter((id) => selectedIds.has(id)).length
  const allChecked: boolean | 'indeterminate' =
    selectedCount === 0 ? false : selectedCount === ids.length ? true : 'indeterminate'

  const toggleAll = (checked: boolean) => {
    onSelectedIdsChange(checked ? new Set(ids) : new Set())
  }
  const toggleRow = (id: string, checked: boolean) => {
    const next = new Set(selectedIds)
    if (checked) next.add(id)
    else next.delete(id)
    onSelectedIdsChange(next)
  }

  const onArrowKeys = (event: KeyboardEvent<HTMLDivElement>) => {
    if (event.key !== 'ArrowDown' && event.key !== 'ArrowUp') return
    const boxes = Array.from(
      regionRef.current?.querySelectorAll<HTMLElement>('tbody [role="checkbox"]') ?? [],
    )
    const index = boxes.findIndex((box) => box === document.activeElement)
    if (index === -1) return
    event.preventDefault()
    boxes[index + (event.key === 'ArrowDown' ? 1 : -1)]?.focus()
  }

  return (
    <div className={className}>
      {/* eslint-disable-next-line jsx-a11y/no-noninteractive-element-interactions */}
      <div
        // Keyboard users must be able to scroll the table area.
        tabIndex={0} // eslint-disable-line jsx-a11y/no-noninteractive-tabindex
        ref={regionRef}
        role="region"
        onKeyDown={onArrowKeys}
        aria-label={t('table.scrollRegion')}
        style={{ maxHeight }}
        className="overflow-auto rounded-lg border border-border bg-surface"
      >
        <table className="w-full border-collapse text-left">
          <caption className="sr-only">{caption}</caption>
          <thead className="sticky top-0 z-10 bg-surface-muted">
            <tr>
              <th scope="col" className="w-14 border-b border-border px-3 py-1">
                <Checkbox
                  hideLabel
                  label={t('table.selectAll')}
                  checked={allChecked}
                  onCheckedChange={toggleAll}
                />
              </th>
              {columns.map((column) => (
                <th
                  key={column.id}
                  scope="col"
                  className={cn(
                    'border-b border-border px-3 py-3 font-semibold',
                    alignClass[column.align ?? 'left'],
                  )}
                >
                  {column.header}
                </th>
              ))}
            </tr>
          </thead>
          <tbody>
            {rows.map((row) => {
              const id = getRowId(row)
              const selected = selectedIds.has(id)
              return (
                <tr
                  key={id}
                  data-selected={selected}
                  className={cn(
                    'border-b border-border last:border-b-0 hover:bg-surface-muted',
                    selected && 'bg-surface-muted',
                  )}
                >
                  <td className="w-14 px-3 py-1">
                    <Checkbox
                      hideLabel
                      label={getRowLabel(row)}
                      checked={selected}
                      onCheckedChange={(checked) => {
                        toggleRow(id, checked)
                      }}
                    />
                  </td>
                  {columns.map((column) => (
                    <td
                      key={column.id}
                      className={cn('px-3 py-2', alignClass[column.align ?? 'left'])}
                    >
                      {column.cell(row)}
                    </td>
                  ))}
                </tr>
              )
            })}
          </tbody>
        </table>
      </div>
      {showSummary ? (
        <p aria-live="polite" className="mt-2 text-fg-muted">
          {t('table.selectedCount', { count: selectedCount, total: ids.length })}
        </p>
      ) : null}
    </div>
  )
}
