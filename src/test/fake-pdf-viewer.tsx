import { useEffect, useState } from 'react'

interface Props {
  load: () => Promise<ArrayBuffer>
  documentKey: string
  title: string
  onPrevious?: () => void
  onNext?: () => void
  hasPrevious?: boolean
  hasNext?: boolean
}

/** Stand-in for the pdf.js viewer in screen tests: loads the bytes and offers the navigation. */
export function PdfViewer({
  load,
  documentKey,
  title,
  onPrevious,
  onNext,
  hasPrevious,
  hasNext,
}: Props) {
  const [bytes, setBytes] = useState<{ key: string; size: number } | null>(null)
  useEffect(() => {
    let alive = true
    load().then(
      (buffer) => {
        if (alive) setBytes({ key: documentKey, size: buffer.byteLength })
      },
      () => undefined,
    )
    return () => {
      alive = false
    }
  }, [load, documentKey])
  return (
    <section aria-label={title}>
      <p data-testid="viewer-doc">{bytes?.key === documentKey ? documentKey : 'loading'}</p>
      <button type="button" disabled={!hasPrevious} onClick={onPrevious}>
        prev
      </button>
      <button type="button" disabled={!hasNext} onClick={onNext}>
        next
      </button>
    </section>
  )
}
