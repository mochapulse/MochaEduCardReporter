import { act, fireEvent, render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import type { ReactElement } from 'react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { expectNoA11yViolations } from '@/test/axe'
import { fakeDocuments, fakeOpenDocument, fakeOpenState, resetFakeEngine } from './fake-engine'
import { PdfViewer, type PdfViewerProps } from './index'

vi.mock('./pdf-engine', () => ({
  openDocument: (bytes: ArrayBuffer) => fakeOpenDocument(bytes),
}))

/** A "PDF" whose page count is its byte length. */
const pages = (count: number) => () => Promise.resolve(new ArrayBuffer(count))

function props(overrides: Partial<PdfViewerProps> = {}): PdfViewerProps {
  return { load: pages(1), documentKey: 'a', title: 'Ana Pérez', ...overrides }
}

async function ready() {
  await screen.findByRole('button', { name: 'Acercar' })
  await waitFor(() => {
    expect(screen.getByRole('button', { name: 'Acercar' })).toBeEnabled()
  })
}

/** Plain render (no wrapper element), so `rerender` keeps the same component instance. */
function renderUi(ui: ReactElement) {
  return { user: userEvent.setup(), ...render(ui) }
}

const percent = () => screen.getByText(/^\d+ %$/).textContent

beforeEach(() => {
  resetFakeEngine()
})
afterEach(() => {
  vi.useRealTimers()
})

describe('states', () => {
  it('shows loading text, then the pages, and announces the document', async () => {
    renderUi(<PdfViewer {...props()} />)
    expect(screen.getByText('Cargando el boletín…')).toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'Acercar' })).toBeDisabled()
    await ready()
    expect(screen.queryByText('Cargando el boletín…')).toBeNull()
    expect(screen.getByRole('region', { name: 'Ana Pérez' })).toBeInTheDocument()
    expect(screen.getByRole('group', { name: 'Página 1' })).toBeInTheDocument()
    expect(screen.getByText('Mostrando boletín de Ana Pérez')).toBeInTheDocument()
    expect(await screen.findByText('Texto de la página 1')).toBeInTheDocument()
  })

  it('shows a plain error with details, and retries', async () => {
    fakeOpenState.fail = new Error('boom: invalid pdf')
    const { user } = renderUi(<PdfViewer {...props()} />)
    expect(await screen.findByText('No se pudo mostrar este boletín.')).toBeInTheDocument()
    expect(screen.getByText('boom: invalid pdf')).toBeInTheDocument()
    expect(screen.getByText('Ver detalles técnicos').closest('details')).not.toHaveAttribute('open')

    fakeOpenState.fail = null
    await user.click(screen.getByRole('button', { name: 'Reintentar' }))
    await ready()
    expect(screen.queryByText('No se pudo mostrar este boletín.')).toBeNull()
  })

  it('treats a rejecting and a throwing load() as errors', async () => {
    const { rerender } = renderUi(
      <PdfViewer {...props({ load: () => Promise.reject(new Error('sin acceso')) })} />,
    )
    expect(await screen.findByText('sin acceso')).toBeInTheDocument()
    rerender(
      <PdfViewer
        {...props({
          documentKey: 'b',
          load: () => {
            throw new Error('lanzado')
          },
        })}
      />,
    )
    expect(await screen.findByText('lanzado')).toBeInTheDocument()
  })

  it('has no accessibility violations while loading, ready and failed', async () => {
    let release: () => void = () => undefined
    fakeOpenState.delay = new Promise<void>((resolve) => {
      release = resolve
    })
    const { container, unmount } = renderUi(<PdfViewer {...props({ load: pages(2) })} />)
    await expectNoA11yViolations(container)
    release()
    await ready()
    await expectNoA11yViolations(container)
    unmount()

    fakeOpenState.delay = null
    fakeOpenState.fail = new Error('x')
    const failed = renderUi(<PdfViewer {...props()} />)
    await screen.findByText('No se pudo mostrar este boletín.')
    await expectNoA11yViolations(failed.container)
  })
})

describe('document changes', () => {
  it('ignores a stale load that finishes after the key changed, and destroys it', async () => {
    let finishFirst: () => void = () => undefined
    const first = new Promise<ArrayBuffer>((resolve) => {
      finishFirst = () => {
        resolve(new ArrayBuffer(3))
      }
    })
    const { rerender } = renderUi(<PdfViewer {...props({ load: () => first })} />)
    rerender(<PdfViewer {...props({ documentKey: 'b', load: pages(1), title: 'Luis' })} />)
    await ready()
    expect(screen.getByText('Mostrando boletín de Luis')).toBeInTheDocument()

    await act(async () => {
      finishFirst()
      await first
    })
    // The stale document (3 pages) never replaced the current one (1 page).
    expect(screen.queryByText('Página 1 de 3')).toBeNull()
    expect(screen.getAllByRole('group', { name: /^Página \d+$/ })).toHaveLength(1)
    await waitFor(() => {
      expect(fakeDocuments.find((d) => d.pageCount === 3)?.destroyed).toBe(true)
    })
  })

  it('destroys the previous document when the key changes, and the last one on unmount', async () => {
    const { rerender, unmount } = renderUi(<PdfViewer {...props()} />)
    await ready()
    const [firstDoc] = fakeDocuments
    rerender(<PdfViewer {...props({ documentKey: 'b', load: pages(2) })} />)
    await screen.findByText('Página 1 de 2')
    expect(firstDoc?.destroyed).toBe(true)
    const current = fakeDocuments.at(-1)
    expect(current?.destroyed).toBe(false)
    unmount()
    expect(current?.destroyed).toBe(true)
  })

  it('keeps the zoom when moving to another document', async () => {
    const { user, rerender } = renderUi(<PdfViewer {...props()} />)
    await ready()
    await user.click(screen.getByRole('button', { name: 'Acercar' }))
    expect(percent()).toBe('110 %')
    rerender(<PdfViewer {...props({ documentKey: 'b' })} />)
    await ready()
    expect(percent()).toBe('110 %')
  })
})

describe('toolbar', () => {
  it('has no page controls for a one-page document and shows them for several', async () => {
    const { unmount } = renderUi(<PdfViewer {...props()} />)
    await ready()
    expect(screen.queryByText(/^Página \d+ de \d+$/)).toBeNull()
    expect(screen.queryByRole('button', { name: 'Página siguiente' })).toBeNull()
    unmount()

    const { user } = renderUi(<PdfViewer {...props({ load: pages(3) })} />)
    expect(await screen.findByText('Página 1 de 3')).toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'Página anterior' })).toBeDisabled()
    expect(screen.getByRole('button', { name: 'Página siguiente' })).toBeEnabled()
    await user.click(screen.getByRole('button', { name: 'Página siguiente' }))
  })

  it('zooms within 50 % and 300 % and disables the buttons at the limits', async () => {
    const { user } = renderUi(<PdfViewer {...props()} />)
    await ready()
    expect(percent()).toBe('100 %') // jsdom has no layout: fit width falls back to 100 %
    const zoomIn = screen.getByRole('button', { name: 'Acercar' })
    const zoomOut = screen.getByRole('button', { name: 'Alejar' })
    for (let i = 0; i < 20 && !zoomIn.hasAttribute('disabled'); i += 1) await user.click(zoomIn)
    expect(percent()).toBe('300 %')
    expect(zoomIn).toBeDisabled()
    for (let i = 0; i < 30 && !zoomOut.hasAttribute('disabled'); i += 1) await user.click(zoomOut)
    expect(percent()).toBe('50 %')
    expect(zoomOut).toBeDisabled()
    await user.click(screen.getByRole('button', { name: 'Ajustar al ancho' }))
    expect(percent()).toBe('100 %')
    expect(screen.getByRole('button', { name: 'Ajustar al ancho' })).toHaveAttribute(
      'aria-pressed',
      'true',
    )
  })

  it('announces zoom changes in a live region', async () => {
    const { user } = renderUi(<PdfViewer {...props()} />)
    await ready()
    await user.click(screen.getByRole('button', { name: 'Acercar' }))
    expect(screen.getByRole('status')).toHaveTextContent('Zoom 110 %')
  })

  it('wires the document buttons and respects hasPrevious / hasNext', async () => {
    const onPrevious = vi.fn()
    const onNext = vi.fn()
    const { user, rerender } = renderUi(
      <PdfViewer {...props({ onPrevious, onNext, hasPrevious: false, hasNext: true })} />,
    )
    await ready()
    expect(screen.getByRole('button', { name: 'Anterior' })).toBeDisabled()
    await user.click(screen.getByRole('button', { name: 'Siguiente' }))
    expect(onNext).toHaveBeenCalledTimes(1)
    rerender(<PdfViewer {...props({ onPrevious, onNext, hasPrevious: true, hasNext: false })} />)
    expect(screen.getByRole('button', { name: 'Siguiente' })).toBeDisabled()
    await user.click(screen.getByRole('button', { name: 'Anterior' }))
    expect(onPrevious).toHaveBeenCalledTimes(1)
  })

  it('shows document buttons only when handlers exist, even while loading', () => {
    const { unmount } = renderUi(<PdfViewer {...props()} />)
    expect(screen.queryByRole('button', { name: 'Siguiente' })).toBeNull()
    unmount()
    renderUi(<PdfViewer {...props({ onNext: vi.fn() })} />)
    expect(screen.getByRole('button', { name: 'Siguiente' })).toBeEnabled()
  })
})

describe('keyboard', () => {
  it('zooms with + - and fits with 0', async () => {
    const { user } = renderUi(<PdfViewer {...props()} />)
    await ready()
    screen.getByRole('group', { name: 'Páginas del boletín' }).focus()
    await user.keyboard('+')
    expect(percent()).toBe('110 %')
    await user.keyboard('-')
    await user.keyboard('-')
    expect(percent()).toBe('90 %')
    await user.keyboard('0')
    expect(percent()).toBe('100 %')
  })

  it('moves between documents with Left / Right only when possible', async () => {
    const onPrevious = vi.fn()
    const onNext = vi.fn()
    const { user, rerender } = renderUi(
      <PdfViewer {...props({ onPrevious, onNext, hasPrevious: false })} />,
    )
    await ready()
    screen.getByRole('group', { name: 'Páginas del boletín' }).focus()
    await user.keyboard('{ArrowLeft}{ArrowRight}')
    expect(onPrevious).not.toHaveBeenCalled()
    expect(onNext).toHaveBeenCalledTimes(1)
    rerender(<PdfViewer {...props({ onPrevious, onNext })} />)
    await user.keyboard('{ArrowLeft}')
    expect(onPrevious).toHaveBeenCalledTimes(1)
  })

  it('scrolls with arrows and PageUp / PageDown', async () => {
    const { user } = renderUi(<PdfViewer {...props({ load: pages(2) })} />)
    await screen.findByText('Página 1 de 2')
    const area = screen.getByRole('group', { name: 'Páginas del boletín' })
    Object.defineProperty(area, 'clientHeight', { configurable: true, value: 500 })
    area.focus()
    await user.keyboard('{ArrowDown}')
    expect(area.scrollTop).toBe(56)
    await user.keyboard('{PageDown}')
    expect(area.scrollTop).toBe(56 + 450)
    await user.keyboard('{ArrowUp}{PageUp}')
    expect(area.scrollTop).toBe(0)
  })

  it('zooms with Ctrl + wheel and leaves a plain wheel alone', async () => {
    renderUi(<PdfViewer {...props()} />)
    await ready()
    const area = screen.getByRole('group', { name: 'Páginas del boletín' })
    expect(fireEvent.wheel(area, { deltaY: -100 })).toBe(true)
    expect(percent()).toBe('100 %')
    expect(fireEvent.wheel(area, { deltaY: -100, ctrlKey: true })).toBe(false)
    expect(percent()).toBe('110 %')
    fireEvent.wheel(area, { deltaY: 100, ctrlKey: true })
    expect(percent()).toBe('100 %')
  })

  it('makes the scroll area focusable', async () => {
    renderUi(<PdfViewer {...props()} />)
    await ready()
    expect(screen.getByRole('group', { name: 'Páginas del boletín' })).toHaveAttribute(
      'tabindex',
      '0',
    )
  })
})

describe('rendering', () => {
  it('draws each page once, redraws debounced after a zoom, and cancels on unmount', async () => {
    vi.useFakeTimers({ shouldAdvanceTime: true })
    const { unmount } = renderUi(<PdfViewer {...props({ load: pages(2) })} />)
    await screen.findByText('Página 1 de 2')
    await act(async () => {
      await vi.advanceTimersByTimeAsync(10)
    })
    const doc = fakeDocuments[0]
    expect(doc?.renders.map((r) => r.page)).toEqual([1, 2])

    fireEvent.click(screen.getByRole('button', { name: 'Acercar' }))
    fireEvent.click(screen.getByRole('button', { name: 'Acercar' }))
    await act(async () => {
      await vi.advanceTimersByTimeAsync(50)
    })
    expect(doc?.renders).toHaveLength(2) // still waiting for the zoom to settle
    await act(async () => {
      await vi.advanceTimersByTimeAsync(200)
    })
    expect(doc?.renders).toHaveLength(4)
    expect(doc?.renders.at(-1)?.scale).toBeCloseTo(1.25)
    unmount()
    expect(doc?.destroyed).toBe(true)
  })
})

describe('lazy rendering', () => {
  afterEach(() => {
    vi.unstubAllGlobals()
  })

  it('draws a page only once it is near the viewport', async () => {
    const observed: {
      callback: (entries: IntersectionObserverEntry[]) => void
      target: Element
    }[] = []
    vi.stubGlobal(
      'IntersectionObserver',
      class {
        constructor(private callback: (entries: IntersectionObserverEntry[]) => void) {}
        observe(target: Element) {
          observed.push({ callback: this.callback, target })
        }
        disconnect() {}
      },
    )
    renderUi(<PdfViewer {...props({ load: pages(2) })} />)
    await screen.findByText('Página 1 de 2')
    const doc = fakeDocuments[0]
    expect(doc?.renders).toEqual([])
    act(() => {
      const second = observed.findLast((o) => o.target.getAttribute('data-page') === '2')
      second?.callback([
        { isIntersecting: true, target: second.target } as IntersectionObserverEntry,
      ])
    })
    await waitFor(() => {
      expect(doc?.renders.map((r) => r.page)).toEqual([2])
    })
  })
})
