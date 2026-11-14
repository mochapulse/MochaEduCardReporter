import { useEffect, useRef, useState } from 'react'
import { t } from '@/i18n/es'
import type { PdfEngineDocument } from './pdf-engine'

/** After a zoom change the page is redrawn only once the zoom has settled. */
export const RENDER_DEBOUNCE_MS = 150

export interface PdfPageProps {
  document: PdfEngineDocument
  pageNumber: number
  /** CSS pixels per PDF point. */
  scale: number
  /** The scrolling element; pages outside it (plus a margin) are not drawn. */
  scrollRoot: HTMLElement | null
}

/**
 * One page: a white sheet of the right size, a canvas (and a transparent text layer)
 * drawn lazily when the page is near the viewport, and its text for screen readers.
 * While a redraw is pending, the previous bitmap stays visible (stretched by CSS), so
 * zooming never flashes blank.
 */
export function PdfPage({ document: doc, pageNumber, scale, scrollRoot }: PdfPageProps) {
  const size = doc.getPageSize(pageNumber)
  const holderRef = useRef<HTMLDivElement>(null)
  const sheetRef = useRef<HTMLDivElement>(null)
  const drawnScale = useRef<number | null>(null)
  const [near, setNear] = useState(typeof IntersectionObserver === 'undefined')
  const [text, setText] = useState('')
  const [failed, setFailed] = useState(false)

  useEffect(() => {
    const sheet = sheetRef.current
    if (!sheet || typeof IntersectionObserver === 'undefined') return
    const observer = new IntersectionObserver(
      (entries) => {
        const last = entries[entries.length - 1]
        if (last) setNear(last.isIntersecting)
      },
      { root: scrollRoot, rootMargin: '100% 0px' },
    )
    observer.observe(sheet)
    return () => {
      observer.disconnect()
    }
  }, [scrollRoot])

  useEffect(() => {
    let current = true
    doc
      .getPageText(pageNumber)
      .then((value) => {
        if (current) setText(value)
      })
      .catch(() => undefined) // The visual page still works; only the hidden text is missing.
    return () => {
      current = false
    }
  }, [doc, pageNumber])

  useEffect(() => {
    const holder = holderRef.current
    if (!near || !holder) return
    if (drawnScale.current === scale) return

    let current = true
    let handle: ReturnType<PdfEngineDocument['renderPage']> | undefined
    const delay = drawnScale.current === null ? 0 : RENDER_DEBOUNCE_MS

    const timer = setTimeout(() => {
      const canvas = window.document.createElement('canvas')
      const textLayer = window.document.createElement('div')
      textLayer.className = 'pdf-text-layer'
      textLayer.setAttribute('aria-hidden', 'true')
      handle = doc.renderPage(pageNumber, canvas, scale, textLayer)
      handle.promise.then(
        () => {
          if (!current) return
          holder.replaceChildren(canvas, textLayer)
          drawnScale.current = scale
          setFailed(false)
        },
        () => {
          if (current) setFailed(true)
        },
      )
    }, delay)

    return () => {
      current = false
      clearTimeout(timer)
      handle?.cancel()
    }
  }, [doc, pageNumber, scale, near])

  return (
    <div
      role="group"
      aria-label={t('pdfViewer.pageLabel', { page: pageNumber })}
      data-page={pageNumber}
      ref={sheetRef}
      className="pdf-sheet relative shrink-0 rounded-sm border border-border shadow-md"
      style={{ width: size.width * scale, height: size.height * scale }}
    >
      <div ref={holderRef} className="pdf-page-holder absolute inset-0" aria-hidden="true" />
      {text ? <p className="sr-only whitespace-pre-line">{text}</p> : null}
      {failed ? (
        <p role="alert" className="absolute inset-x-0 top-1/2 px-4 text-center text-base">
          {t('pdfViewer.pageFailed')}
        </p>
      ) : null}
    </div>
  )
}
