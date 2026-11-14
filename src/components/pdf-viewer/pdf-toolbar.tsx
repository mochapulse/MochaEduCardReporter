import {
  ChevronDown,
  ChevronLeft,
  ChevronRight,
  ChevronUp,
  Maximize2,
  ZoomIn,
  ZoomOut,
} from 'lucide-react'
import { Button } from '@/components/ui/button'
import { t } from '@/i18n/es'

export interface PdfToolbarProps {
  /** False while loading or after an error: zoom and page controls are disabled. */
  ready: boolean
  zoomPercent: number
  canZoomIn: boolean
  canZoomOut: boolean
  isFitWidth: boolean
  onZoomIn: () => void
  onZoomOut: () => void
  onFitWidth: () => void
  pageCount: number
  currentPage: number
  onPreviousPage: () => void
  onNextPage: () => void
  onPrevious?: (() => void) | undefined
  onNext?: (() => void) | undefined
  hasPrevious?: boolean | undefined
  hasNext?: boolean | undefined
}

/** Always visible: text + icon buttons, each at least 44 x 44 px (design-system Button). */
export function PdfToolbar(props: PdfToolbarProps) {
  const { ready, onPrevious, onNext } = props
  const showDocumentButtons = onPrevious !== undefined || onNext !== undefined

  return (
    <div
      role="toolbar"
      aria-label={t('pdfViewer.toolbarLabel')}
      className="flex flex-wrap items-center gap-x-6 gap-y-2 border-b border-border bg-surface px-3 py-2"
    >
      {showDocumentButtons ? (
        <div role="group" aria-label={t('pdfViewer.documentsLabel')} className="flex gap-2">
          <Button
            variant="outline"
            icon={ChevronLeft}
            label={t('pdfViewer.previousDocument')}
            disabled={onPrevious === undefined || props.hasPrevious === false}
            onClick={onPrevious}
          />
          <Button
            variant="outline"
            icon={ChevronRight}
            label={t('pdfViewer.nextDocument')}
            disabled={onNext === undefined || props.hasNext === false}
            onClick={onNext}
          />
        </div>
      ) : null}

      <div className="flex flex-wrap items-center gap-2">
        <Button
          variant="outline"
          icon={ZoomOut}
          label={t('pdfViewer.zoomOut')}
          disabled={!ready || !props.canZoomOut}
          onClick={props.onZoomOut}
        />
        <span className="min-w-16 text-center font-semibold tabular-nums">
          {t('pdfViewer.zoomValue', { percent: props.zoomPercent })}
        </span>
        <Button
          variant="outline"
          icon={ZoomIn}
          label={t('pdfViewer.zoomIn')}
          disabled={!ready || !props.canZoomIn}
          onClick={props.onZoomIn}
        />
        <Button
          variant="outline"
          icon={Maximize2}
          label={t('pdfViewer.fitWidth')}
          aria-pressed={props.isFitWidth}
          disabled={!ready}
          onClick={props.onFitWidth}
        />
      </div>

      {ready && props.pageCount > 1 ? (
        <div className="flex flex-wrap items-center gap-2">
          <Button
            variant="outline"
            icon={ChevronUp}
            label={t('pdfViewer.previousPage')}
            disabled={props.currentPage <= 1}
            onClick={props.onPreviousPage}
          />
          <span className="font-semibold">
            {t('pdfViewer.page', { page: props.currentPage, total: props.pageCount })}
          </span>
          <Button
            variant="outline"
            icon={ChevronDown}
            label={t('pdfViewer.nextPage')}
            disabled={props.currentPage >= props.pageCount}
            onClick={props.onNextPage}
          />
        </div>
      ) : null}
    </div>
  )
}
