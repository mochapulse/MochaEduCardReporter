import { Loader2 } from 'lucide-react'
import {
  useCallback,
  useEffect,
  useLayoutEffect,
  useRef,
  useState,
  type KeyboardEvent as ReactKeyboardEvent,
} from 'react'
import { Button } from '@/components/ui/button'
import { t } from '@/i18n/es'
import { cn } from '@/lib/utils'
import { PdfPage } from './pdf-page'
import { PdfToolbar } from './pdf-toolbar'
import { usePdfDocument } from './use-pdf-document'
import {
  FIT_WIDTH,
  MAX_ZOOM,
  MIN_ZOOM,
  fitWidthScale,
  toPercent,
  zoomInFrom,
  zoomOutFrom,
  type ZoomMode,
} from './zoom'
import './pdf-viewer.css'

export interface PdfViewerProps {
  /** Loads the bytes of the document to show. A new function identity or `documentKey` reloads. */
  load: () => Promise<ArrayBuffer>
  /** Stable key of the current document (e.g. its path). */
  documentKey: string
  /** Accessible name, e.g. the student name. */
  title: string
  /** Optional: shown in the toolbar to move between documents. */
  onPrevious?: () => void
  onNext?: () => void
  hasPrevious?: boolean
  hasNext?: boolean
  className?: string
}

/** Space around the pages inside the scroll area, in CSS pixels. */
const PAGE_PADDING = 16
const ARROW_SCROLL_PX = 56
const EPS = 0.005

export function PdfViewer({
  load,
  documentKey,
  title,
  onPrevious,
  onNext,
  hasPrevious,
  hasNext,
  className,
}: PdfViewerProps) {
  const { state, retry } = usePdfDocument(load, documentKey)
  const doc = state.status === 'ready' ? state.document : null

  const [scrollEl, setScrollEl] = useState<HTMLDivElement | null>(null)
  const [availableWidth, setAvailableWidth] = useState(0)
  const [zoom, setZoom] = useState<ZoomMode>(FIT_WIDTH)
  const [zoomNote, setZoomNote] = useState<{ doc: object; text: string } | null>(null)
  const [pageState, setPageState] = useState<{ doc: object | null; page: number }>({
    doc: null,
    page: 1,
  })
  const scrollRatio = useRef(0)
  const scrollRef = useRef<HTMLDivElement | null>(null)
  const attachScroll = useCallback((el: HTMLDivElement | null) => {
    scrollRef.current = el
    setScrollEl(el)
  }, [])
  const setScrollTop = useCallback((top: number) => {
    if (scrollRef.current) scrollRef.current.scrollTop = top
  }, [])

  // Measure the scroll area; "fit width" follows its width.
  useLayoutEffect(() => {
    if (!scrollEl) return
    const measure = () => {
      setAvailableWidth(Math.max(0, scrollEl.clientWidth - 2 * PAGE_PADDING))
    }
    measure()
    const observer = new ResizeObserver(measure)
    observer.observe(scrollEl)
    return () => {
      observer.disconnect()
    }
  }, [scrollEl])

  const pageCount = doc?.pageCount ?? 0
  const widestPage = doc
    ? Math.max(...Array.from({ length: pageCount }, (_, i) => doc.getPageSize(i + 1).width))
    : 0
  const fitScale = fitWidthScale(availableWidth, widestPage)
  const scale = zoom.kind === 'fit' ? fitScale : zoom.scale
  const percent = toPercent(scale)

  // A new document starts at the top.
  useLayoutEffect(() => {
    setScrollTop(0)
    scrollRatio.current = 0
  }, [doc, setScrollTop])

  // Keep the reading position when the page height changes with the zoom.
  useLayoutEffect(() => {
    if (scrollEl) setScrollTop(scrollRatio.current * scrollEl.scrollHeight)
  }, [scale, scrollEl, setScrollTop])

  const currentPage = pageState.doc === doc ? pageState.page : 1

  const updateCurrentPage = useCallback(() => {
    if (!scrollEl || !doc) return
    if (scrollEl.scrollHeight > 0) scrollRatio.current = scrollEl.scrollTop / scrollEl.scrollHeight
    const probe = scrollEl.scrollTop + scrollEl.clientHeight * 0.4
    let page = 1
    for (const el of scrollEl.querySelectorAll<HTMLElement>('[data-page]')) {
      if (el.offsetTop <= probe) page = Number(el.dataset['page'])
    }
    setPageState((previous) =>
      previous.doc === doc && previous.page === page ? previous : { doc, page },
    )
  }, [scrollEl, doc])

  const goToPage = useCallback(
    (page: number) => {
      if (!scrollEl) return
      const target = scrollEl.querySelector<HTMLElement>(`[data-page="${String(page)}"]`)
      if (target) setScrollTop(target.offsetTop - PAGE_PADDING)
    },
    [scrollEl, setScrollTop],
  )

  const applyZoom = useCallback(
    (mode: ZoomMode, nextScale: number) => {
      setZoom(mode)
      if (doc) {
        const text =
          mode.kind === 'fit'
            ? t('pdfViewer.fitWidthAnnouncement', { percent: toPercent(nextScale) })
            : t('pdfViewer.zoomAnnouncement', { percent: toPercent(nextScale) })
        setZoomNote({ doc, text })
      }
    },
    [doc],
  )

  const zoomIn = useCallback(() => {
    const next = Math.min(MAX_ZOOM, zoomInFrom(scale))
    applyZoom({ kind: 'custom', scale: next }, next)
  }, [scale, applyZoom])

  const zoomOut = useCallback(() => {
    const next = Math.max(MIN_ZOOM, zoomOutFrom(scale))
    applyZoom({ kind: 'custom', scale: next }, next)
  }, [scale, applyZoom])

  const fitWidth = useCallback(() => {
    applyZoom(FIT_WIDTH, fitScale)
  }, [fitScale, applyZoom])

  // Ctrl + wheel zooms. Must be a non-passive native listener to cancel the page zoom.
  useEffect(() => {
    if (!scrollEl || !doc) return
    const onWheel = (event: WheelEvent) => {
      if (!event.ctrlKey) return
      event.preventDefault()
      if (event.deltaY < 0) zoomIn()
      else if (event.deltaY > 0) zoomOut()
    }
    scrollEl.addEventListener('wheel', onWheel, { passive: false })
    return () => {
      scrollEl.removeEventListener('wheel', onWheel)
    }
  }, [scrollEl, doc, zoomIn, zoomOut])

  const canPrevious = onPrevious !== undefined && hasPrevious !== false
  const canNext = onNext !== undefined && hasNext !== false

  const onKeyDown = (event: ReactKeyboardEvent<HTMLDivElement>) => {
    if (event.ctrlKey || event.altKey || event.metaKey) return
    // Native controls keep their own keys (the "details" summary, buttons use Enter/Space).
    const scrollBy = (dy: number) => {
      if (scrollEl) setScrollTop(scrollEl.scrollTop + dy)
      event.preventDefault()
    }
    switch (event.key) {
      case '+':
      case '=':
        if (doc) zoomIn()
        event.preventDefault()
        break
      case '-':
        if (doc) zoomOut()
        event.preventDefault()
        break
      case '0':
        if (doc) fitWidth()
        event.preventDefault()
        break
      case 'ArrowDown':
        scrollBy(ARROW_SCROLL_PX)
        break
      case 'ArrowUp':
        scrollBy(-ARROW_SCROLL_PX)
        break
      case 'PageDown':
        scrollBy((scrollEl?.clientHeight ?? 0) * 0.9)
        break
      case 'PageUp':
        scrollBy(-(scrollEl?.clientHeight ?? 0) * 0.9)
        break
      case 'ArrowLeft':
        if (canPrevious) {
          event.preventDefault()
          onPrevious()
        }
        break
      case 'ArrowRight':
        if (canNext) {
          event.preventDefault()
          onNext()
        }
        break
      default:
        break
    }
  }

  const announcement =
    zoomNote && zoomNote.doc === doc ? zoomNote.text : doc ? t('pdfViewer.showing', { title }) : ''

  return (
    // The keyboard shortcuts apply whenever focus is anywhere inside the viewer.
    // eslint-disable-next-line jsx-a11y/no-noninteractive-element-interactions
    <section
      aria-label={title}
      onKeyDown={onKeyDown}
      className={cn(
        'flex min-h-0 flex-col overflow-hidden rounded-lg border border-border bg-surface',
        className,
      )}
    >
      <PdfToolbar
        ready={doc !== null}
        zoomPercent={percent}
        canZoomIn={percent < toPercent(MAX_ZOOM) - EPS}
        canZoomOut={percent > toPercent(MIN_ZOOM) + EPS}
        isFitWidth={zoom.kind === 'fit'}
        onZoomIn={zoomIn}
        onZoomOut={zoomOut}
        onFitWidth={fitWidth}
        pageCount={pageCount}
        currentPage={currentPage}
        onPreviousPage={() => {
          goToPage(currentPage - 1)
        }}
        onNextPage={() => {
          goToPage(currentPage + 1)
        }}
        onPrevious={onPrevious}
        onNext={onNext}
        hasPrevious={hasPrevious}
        hasNext={hasNext}
      />

      <div
        ref={attachScroll}
        role="group"
        aria-label={t('pdfViewer.pagesLabel')}
        // The scroll area must be reachable by keyboard so it can be scrolled.
        // eslint-disable-next-line jsx-a11y/no-noninteractive-tabindex
        tabIndex={0}
        onScroll={updateCurrentPage}
        className="pdf-scroll min-h-0 flex-1 bg-surface-muted"
      >
        {state.status === 'loading' ? (
          <div
            role="status"
            className="flex h-full min-h-40 items-center justify-center gap-3 p-6 text-lg"
          >
            <Loader2 aria-hidden="true" className="size-6 motion-safe:animate-spin" />
            <span>{t('pdfViewer.loading')}</span>
          </div>
        ) : null}

        {state.status === 'error' ? (
          <div role="alert" className="mx-auto flex max-w-xl flex-col items-start gap-4 p-6">
            <p className="text-lg font-semibold">{t('pdfViewer.failed')}</p>
            <p>{t('pdfViewer.failedHelp')}</p>
            <Button variant="primary" label={t('pdfViewer.retry')} onClick={retry} />
            <details className="w-full">
              <summary className="min-h-11 cursor-pointer py-2 font-semibold">
                {t('pdfViewer.technicalDetails')}
              </summary>
              <pre className="mt-2 rounded-md border border-border bg-surface p-3 text-sm break-words whitespace-pre-wrap">
                {state.message}
              </pre>
            </details>
          </div>
        ) : null}

        {doc ? (
          <div
            className="flex min-w-fit flex-col items-center gap-4"
            style={{ padding: PAGE_PADDING }}
          >
            {Array.from({ length: doc.pageCount }, (_, i) => (
              <PdfPage
                key={i + 1}
                document={doc}
                pageNumber={i + 1}
                scale={scale}
                scrollRoot={scrollEl}
              />
            ))}
          </div>
        ) : null}
      </div>

      <div role="status" className="sr-only">
        {announcement}
      </div>
    </section>
  )
}
