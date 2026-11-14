import '@testing-library/jest-dom/vitest'
import { cleanup } from '@testing-library/react'
import { afterEach } from 'vitest'

afterEach(() => {
  cleanup()
  delete document.documentElement.dataset['theme']
  delete document.documentElement.dataset['textSize']
})

// jsdom does not implement matchMedia; tests drive it through setSystemDark().
let systemDark = false
const listeners = new Set<(event: MediaQueryListEvent) => void>()

export function setSystemDark(dark: boolean): void {
  systemDark = dark
  for (const listener of listeners) listener({ matches: dark } as MediaQueryListEvent)
}

window.matchMedia = (query: string): MediaQueryList =>
  ({
    get matches() {
      return query.includes('dark') ? systemDark : false
    },
    media: query,
    addEventListener: (_: string, listener: (event: MediaQueryListEvent) => void) => {
      listeners.add(listener)
    },
    removeEventListener: (_: string, listener: (event: MediaQueryListEvent) => void) => {
      listeners.delete(listener)
    },
  }) as unknown as MediaQueryList

// jsdom lacks these APIs that Radix primitives call.
window.HTMLElement.prototype.scrollIntoView = () => undefined
window.HTMLElement.prototype.hasPointerCapture = () => false
window.HTMLElement.prototype.releasePointerCapture = () => undefined
globalThis.ResizeObserver = class {
  observe() {}
  unobserve() {}
  disconnect() {}
}
