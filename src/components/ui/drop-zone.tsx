import { FileSpreadsheet } from 'lucide-react'
import { useId, useRef, useState, type DragEvent } from 'react'
import { t } from '@/i18n/es'
import { cn } from '@/lib/utils'
import { Button } from './button'

export interface DropZoneProps {
  /** Called when the visible button is pressed (opens the native file picker). */
  onChooseFile: () => void
  /** Called after a drop with the supported files and the ones that were rejected. */
  onDropFiles?: (accepted: File[], rejected: File[]) => void
  /** Supported extensions, lowercase with the dot. */
  acceptedExtensions?: readonly string[]
  /** Forces the drag-over look (gallery and tests only). */
  forceDragOver?: boolean
  className?: string
}

const DEFAULT_EXTENSIONS = ['.xlsx', '.xls', '.csv'] as const

/** Large target for the gradebook file. The button is always present: drop is never the only way. */
export function DropZone({
  onChooseFile,
  onDropFiles,
  acceptedExtensions = DEFAULT_EXTENSIONS,
  forceDragOver = false,
  className,
}: DropZoneProps) {
  const [depth, setDepth] = useState(0)
  const descriptionId = useId()
  const active = forceDragOver || depth > 0
  const dragCounter = useRef(0)

  const isSupported = (file: File) =>
    acceptedExtensions.some((extension) => file.name.toLowerCase().endsWith(extension))

  const enter = (event: DragEvent) => {
    event.preventDefault()
    dragCounter.current += 1
    setDepth(dragCounter.current)
  }
  const leave = (event: DragEvent) => {
    event.preventDefault()
    dragCounter.current = Math.max(0, dragCounter.current - 1)
    setDepth(dragCounter.current)
  }
  const drop = (event: DragEvent) => {
    event.preventDefault()
    dragCounter.current = 0
    setDepth(0)
    const files = Array.from(event.dataTransfer.files)
    onDropFiles?.(
      files.filter(isSupported),
      files.filter((file) => !isSupported(file)),
    )
  }

  return (
    <section
      aria-labelledby={descriptionId}
      data-drag-over={active}
      onDragEnter={enter}
      onDragLeave={leave}
      onDragOver={(event) => {
        event.preventDefault()
      }}
      onDrop={drop}
      className={cn(
        'flex flex-col items-center gap-4 rounded-lg border-4 border-dashed p-10 text-center transition-colors',
        active ? 'border-primary bg-surface-muted' : 'border-fg-muted bg-surface',
        className,
      )}
    >
      <FileSpreadsheet aria-hidden="true" className="size-14 text-primary" />
      <p id={descriptionId} className="text-2xl font-semibold">
        {active ? t('dropZone.dragOver') : t('dropZone.title')}
      </p>
      <p className="text-fg-muted">{t('dropZone.or')}</p>
      <Button label={t('dropZone.button')} onClick={onChooseFile} className="px-8 text-lg" />
      <p className="text-sm text-fg-muted">
        {t('dropZone.formats', { formats: acceptedExtensions.join(', ') })}
      </p>
    </section>
  )
}
