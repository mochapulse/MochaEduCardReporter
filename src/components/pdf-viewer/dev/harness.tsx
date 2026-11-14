/**
 * Dev-only page that mounts the real PdfViewer with real sample reports. It is built
 * by `vite.harness.config.ts` (never by the app build) and served by a server that
 * applies the production Content-Security-Policy as a response header.
 *
 * Query parameters: theme=dark, doc=<index>, zoomIn=<clicks>, zoomOut=<clicks>,
 * log=1 (shows the error log under the viewer).
 * Everything that goes wrong (errors, rejections, CSP violations) is written into
 * the element #harness-log so a headless run can read it with --dump-dom.
 */
import { StrictMode, useCallback, useEffect, useMemo, useState } from 'react'
import { createRoot } from 'react-dom/client'
import { t } from '@/i18n/es'
import { PdfViewer } from '../index'
import '@/styles/globals.css'
import manifest from './samples/manifest.json'

const urls = import.meta.glob<string>('./samples/*.pdf', { query: '?url', import: 'default' })

interface Sample {
  file: string
  name: string
}
const samples: Sample[] = manifest

const params = new URLSearchParams(window.location.search)
const theme = params.get('theme')
if (theme) document.documentElement.dataset['theme'] = theme

const logLines: string[] = []
function log(kind: string, detail: string): void {
  logLines.push(`${kind}: ${detail}`)
  const target = document.getElementById('harness-log')
  if (target) target.textContent = logLines.join('\n')
}
window.addEventListener('error', (event) => {
  log('error', event.message)
})
window.addEventListener('unhandledrejection', (event) => {
  log('rejection', String(event.reason))
})
document.addEventListener('securitypolicyviolation', (event) => {
  log('csp', `${event.violatedDirective} blocked ${event.blockedURI}`)
})
const originalError = console.error.bind(console)
console.error = (...args: unknown[]) => {
  log('console.error', args.map(String).join(' '))
  originalError(...args)
}

function clickTimes(label: string, count: number): void {
  for (let i = 0; i < count; i += 1) {
    const button = [...document.querySelectorAll('button')].find((b) => b.textContent === label)
    button?.click()
  }
}

function Harness() {
  const [index, setIndex] = useState(() => Number(params.get('doc') ?? 0))
  const sample = samples[index]
  const file = sample?.file ?? ''

  const load = useMemo(
    () => async () => {
      const loadUrl = urls[`./samples/${file}`]
      if (!loadUrl) throw new Error(file)
      const response = await fetch(await loadUrl())
      return response.arrayBuffer()
    },
    [file],
  )

  const previous = useCallback(() => {
    setIndex((i) => Math.max(0, i - 1))
  }, [])
  const next = useCallback(() => {
    setIndex((i) => Math.min(samples.length - 1, i + 1))
  }, [])

  useEffect(() => {
    const timer = setTimeout(() => {
      clickTimes(t('pdfViewer.zoomIn'), Number(params.get('zoomIn') ?? 0))
      clickTimes(t('pdfViewer.zoomOut'), Number(params.get('zoomOut') ?? 0))
    }, 1200)
    return () => {
      clearTimeout(timer)
    }
  }, [])

  return (
    <div className="flex h-screen flex-col gap-3 p-4">
      <nav className="flex flex-wrap gap-2">
        {samples.map((item, i) => (
          <button
            key={item.file}
            type="button"
            className="min-h-11 rounded-md border-2 border-primary px-4"
            onClick={() => {
              setIndex(i)
            }}
          >
            {item.name}
          </button>
        ))}
      </nav>
      <PdfViewer
        className="flex-1"
        load={load}
        documentKey={file}
        title={sample?.name ?? ''}
        onPrevious={previous}
        onNext={next}
        hasPrevious={index > 0}
        hasNext={index < samples.length - 1}
      />
      <pre id="harness-log" className={params.has('log') ? 'text-sm' : 'sr-only'} />
    </div>
  )
}

const root = document.getElementById('root')
if (!root) throw new Error('missing root')
createRoot(root).render(
  <StrictMode>
    <Harness />
  </StrictMode>,
)
