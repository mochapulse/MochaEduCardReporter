import { useCallback, useEffect, useState } from 'react'
import { openDocument, type PdfEngineDocument } from './pdf-engine'

export type PdfDocumentState =
  | { readonly status: 'loading' }
  | { readonly status: 'error'; readonly message: string }
  | { readonly status: 'ready'; readonly document: PdfEngineDocument }

type Result =
  | { readonly status: 'error'; readonly message: string }
  | { readonly status: 'ready'; readonly document: PdfEngineDocument }

interface Settled {
  /** Identifies the request this result belongs to; stale results are never shown. */
  readonly load: () => Promise<ArrayBuffer>
  readonly documentKey: string
  readonly attempt: number
  readonly result: Result
}

function describeError(error: unknown): string {
  if (error instanceof Error) return error.message || error.name
  return String(error)
}

/**
 * Loads and opens a document. A new `load` identity, a new `documentKey` or `retry()`
 * starts a new request; answers of older requests are dropped (and their pdf.js
 * document destroyed). The document is destroyed when it is replaced or on unmount.
 */
export function usePdfDocument(
  load: () => Promise<ArrayBuffer>,
  documentKey: string,
): { state: PdfDocumentState; retry: () => void } {
  const [attempt, setAttempt] = useState(0)
  const [settled, setSettled] = useState<Settled | null>(null)

  useEffect(() => {
    let current = true
    let opened: PdfEngineDocument | undefined

    const settle = (result: Result) => {
      setSettled({ load, documentKey, attempt, result })
    }

    // Promise.resolve().then also turns a synchronous throw of `load` into a rejection.
    Promise.resolve()
      .then(load)
      .then(openDocument)
      .then(
        (document) => {
          if (current) {
            opened = document
            settle({ status: 'ready', document })
          } else {
            document.destroy()
          }
        },
        (error: unknown) => {
          if (current) settle({ status: 'error', message: describeError(error) })
        },
      )

    return () => {
      current = false
      opened?.destroy()
    }
  }, [load, documentKey, attempt])

  const retry = useCallback(() => {
    setAttempt((value) => value + 1)
  }, [])

  const isCurrent =
    settled?.load === load && settled.documentKey === documentKey && settled.attempt === attempt
  // Derived on purpose: the moment the inputs change, the old document is no longer shown.
  const state: PdfDocumentState = isCurrent ? settled.result : { status: 'loading' }
  return { state, retry }
}
