/** Test double for ./pdf-engine: records every call so tests can assert cleanup. */
import type { PdfEngineDocument, PdfPageSize, PdfRenderHandle } from './pdf-engine'

export interface FakeDocument extends PdfEngineDocument {
  destroyed: boolean
  renders: { page: number; scale: number; cancelled: boolean }[]
}

export const fakeDocuments: FakeDocument[] = []
export const fakeOpenState: { fail: Error | null; delay: Promise<void> | null } = {
  fail: null,
  delay: null,
}

export function resetFakeEngine(): void {
  fakeDocuments.length = 0
  fakeOpenState.fail = null
  fakeOpenState.delay = null
}

export function makeFakeDocument(pageCount: number): FakeDocument {
  const size: PdfPageSize = { width: 595, height: 842 }
  const doc: FakeDocument = {
    pageCount,
    destroyed: false,
    renders: [],
    getPageSize: () => size,
    renderPage(page, _canvas, scale): PdfRenderHandle {
      const record = { page, scale, cancelled: false }
      doc.renders.push(record)
      return {
        promise: Promise.resolve(),
        cancel: () => {
          record.cancelled = true
        },
      }
    },
    getPageText: (page) => Promise.resolve(`Texto de la página ${String(page)}`),
    destroy: () => {
      doc.destroyed = true
    },
  }
  return doc
}

/** `openDocument` stand-in: the page count is the byte length of the buffer. */
export async function fakeOpenDocument(bytes: ArrayBuffer): Promise<PdfEngineDocument> {
  if (fakeOpenState.delay) await fakeOpenState.delay
  if (fakeOpenState.fail) throw fakeOpenState.fail
  const doc = makeFakeDocument(bytes.byteLength)
  fakeDocuments.push(doc)
  return doc
}
