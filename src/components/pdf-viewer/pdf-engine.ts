/**
 * Thin adapter over pdf.js. It is the ONLY module that imports pdf.js, so the React
 * component can be tested against a fake of this module (pdf.js cannot render in jsdom).
 *
 * Offline / CSP notes (see PLAN_REDO_CODE.md 3.6 and 3.7):
 * - The worker is bundled by Vite (`?url` import) and served from the app origin,
 *   so `worker-src 'self'` is enough.
 * - The reports use Helvetica / Helvetica-Bold without embedding them. pdf.js
 *   substitutes its own Liberation Sans files, which ship in `public/pdfjs/standard_fonts/`
 *   and are fetched from the app origin (`connect-src 'self'`). `useSystemFonts: false`
 *   makes Windows and Linux render with the same font instead of whatever the OS has.
 * - `useWasm: false` keeps pdf.js away from
 *   WebAssembly, which the CSP (`script-src 'self'`) does not allow (this pdf.js version has no eval). Wasm only speeds up
 *   image decoding, and the reports contain no images.
 * - CMaps are not shipped: the reports only use WinAnsi standard fonts.
 * - The "legacy" pdf.js build is used because the Linux web view (WebKitGTK) can be older
 *   than what the modern build assumes.
 */
import type {
  PDFDocumentProxy,
  PDFWorker,
  RenderTask,
  TextLayer,
} from 'pdfjs-dist/legacy/build/pdf.mjs'
import type * as PdfJsModule from 'pdfjs-dist/legacy/build/pdf.mjs'
import workerUrl from 'pdfjs-dist/legacy/build/pdf.worker.min.mjs?url'

/** Page size in PDF points (1/72 inch) at zoom 100 %. */
export interface PdfPageSize {
  readonly width: number
  readonly height: number
}

export interface PdfRenderHandle {
  /** Resolves when drawing finished OR was cancelled; rejects only on real errors. */
  readonly promise: Promise<void>
  cancel: () => void
}

export interface PdfEngineDocument {
  readonly pageCount: number
  /** Size of a page (1-based) at scale 1. Known as soon as the document is open. */
  getPageSize: (pageNumber: number) => PdfPageSize
  /**
   * Draws a page into `canvas` at `scale` (CSS pixels per PDF point), using
   * devicePixelRatio for the bitmap resolution. When `textLayer` is given, the
   * selectable text is laid out inside it. The canvas is NOT given a CSS size.
   */
  renderPage: (
    pageNumber: number,
    canvas: HTMLCanvasElement,
    scale: number,
    textLayer?: HTMLElement,
  ) => PdfRenderHandle
  /** Plain text of a page (1-based), for assistive technology. */
  getPageText: (pageNumber: number) => Promise<string>
  /** Releases the document, its render tasks and its memory. Safe to call twice. */
  destroy: () => void
}

/** Upper bound for a canvas bitmap; keeps 300 % on a high-DPI screen within browser limits. */
const MAX_CANVAS_PIXELS = 16_000_000

const FONT_DATA_PATH = 'pdfjs/standard_fonts/'

type PdfJs = typeof PdfJsModule

let pdfjsPromise: Promise<PdfJs> | undefined
let sharedWorker: PDFWorker | undefined

function loadPdfJs(): Promise<PdfJs> {
  pdfjsPromise ??= import('pdfjs-dist/legacy/build/pdf.mjs').then((pdfjs) => {
    pdfjs.GlobalWorkerOptions.workerSrc = workerUrl
    return pdfjs
  })
  return pdfjsPromise
}

/** One worker is reused for all documents; it is recreated if it was destroyed. */
function getWorker(pdfjs: PdfJs): PDFWorker {
  if (!sharedWorker || sharedWorker.destroyed) sharedWorker = new pdfjs.PDFWorker()
  return sharedWorker
}

function standardFontDataUrl(): string {
  const base = import.meta.env.BASE_URL
  return new URL(`${base}${FONT_DATA_PATH}`, document.baseURI).href
}

function isCancellation(error: unknown): boolean {
  return error instanceof Error && error.name === 'RenderingCancelledException'
}

export async function openDocument(bytes: ArrayBuffer): Promise<PdfEngineDocument> {
  const pdfjs = await loadPdfJs()
  // pdf.js transfers the buffer to its worker, which would detach the caller's copy.
  const data = new Uint8Array(bytes.slice(0))
  const loadingTask = pdfjs.getDocument({
    data,
    worker: getWorker(pdfjs),
    standardFontDataUrl: standardFontDataUrl(),
    useSystemFonts: false,
    useWorkerFetch: true,
    useWasm: false,
  })

  let pdf: PDFDocumentProxy
  try {
    pdf = await loadingTask.promise
  } catch (error) {
    await loadingTask.destroy()
    throw error
  }

  try {
    const sizes: PdfPageSize[] = []
    for (let n = 1; n <= pdf.numPages; n += 1) {
      const view = (await pdf.getPage(n)).getViewport({ scale: 1 })
      sizes.push({ width: view.width, height: view.height })
    }
    return createDocument(pdfjs, pdf, loadingTask.destroy.bind(loadingTask), sizes)
  } catch (error) {
    await loadingTask.destroy()
    throw error
  }
}

function createDocument(
  pdfjs: PdfJs,
  pdf: PDFDocumentProxy,
  destroyTask: () => Promise<void>,
  sizes: readonly PdfPageSize[],
): PdfEngineDocument {
  const textCache = new Map<number, Promise<string>>()
  let destroyed = false

  const sizeOf = (pageNumber: number): PdfPageSize => {
    const size = sizes[pageNumber - 1]
    if (!size) throw new RangeError(`Page ${String(pageNumber)} does not exist`)
    return size
  }

  return {
    pageCount: pdf.numPages,
    getPageSize: sizeOf,

    renderPage(pageNumber, canvas, scale, textLayerContainer) {
      sizeOf(pageNumber)
      // An object, not a `let`: it is mutated from `cancel()` while `draw()` is suspended.
      const flags = { cancelled: false }
      // A function call, so TypeScript does not keep a stale narrowing across `await`.
      const isCancelled = (): boolean => flags.cancelled
      let renderTask: RenderTask | undefined
      let textLayer: TextLayer | undefined

      const draw = async (): Promise<void> => {
        const page = await pdf.getPage(pageNumber)
        if (isCancelled()) return
        const viewport = page.getViewport({ scale })

        const wanted = window.devicePixelRatio || 1
        const area = viewport.width * viewport.height
        const output = Math.min(wanted, Math.sqrt(MAX_CANVAS_PIXELS / area))
        canvas.width = Math.max(1, Math.floor(viewport.width * output))
        canvas.height = Math.max(1, Math.floor(viewport.height * output))

        renderTask = page.render({
          canvas,
          viewport,
          ...(output === 1 ? {} : { transform: [output, 0, 0, output, 0, 0] }),
        })
        try {
          await renderTask.promise
        } catch (error) {
          if (isCancellation(error)) return
          throw error
        }
        if (isCancelled() || !textLayerContainer) return

        textLayerContainer.style.setProperty('--total-scale-factor', String(scale))
        textLayer = new pdfjs.TextLayer({
          textContentSource: page.streamTextContent(),
          container: textLayerContainer,
          viewport,
        })
        try {
          await textLayer.render()
        } catch (error) {
          // A cancelled text layer must not surface as an error.
          if (!isCancelled()) throw error
        }
      }

      return {
        promise: draw(),
        cancel() {
          flags.cancelled = true
          renderTask?.cancel()
          textLayer?.cancel()
        },
      }
    },

    getPageText(pageNumber) {
      sizeOf(pageNumber)
      let cached = textCache.get(pageNumber)
      if (!cached) {
        cached = pdf
          .getPage(pageNumber)
          .then((page) => page.getTextContent())
          .then((content) => {
            let text = ''
            for (const item of content.items) {
              // Separate table cells and words that pdf.js reports as distinct items.
              if ('str' in item) text += item.hasEOL ? `${item.str}\n` : `${item.str} `
            }
            return text.replace(/ {2,}/g, ' ').replace(/ \n/g, '\n').trim()
          })
        textCache.set(pageNumber, cached)
      }
      return cached
    },

    destroy() {
      if (destroyed) return
      destroyed = true
      void destroyTask().catch(() => undefined)
    },
  }
}
