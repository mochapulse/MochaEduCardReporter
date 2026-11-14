import { act, render } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it, vi } from 'vitest'
import { setSystemDark } from '@/test/setup'
import { createLocalStorageStore, createMemoryStore, readChoice } from './preferences'
import { TextSizeProvider, textSizeScale, useTextSize } from './text-size'
import { ThemeProvider, useTheme } from './theme'

function Probe() {
  const { preference, resolved, setPreference } = useTheme()
  const { size, setSize } = useTextSize()
  return (
    <div>
      <p data-testid="theme">{`${preference}/${resolved}`}</p>
      <p data-testid="size">{size}</p>
      <button
        type="button"
        onClick={() => {
          setPreference('oscuro')
        }}
      >
        oscuro
      </button>
      <button
        type="button"
        onClick={() => {
          setPreference('sistema')
        }}
      >
        sistema
      </button>
      <button
        type="button"
        onClick={() => {
          setSize('muy-grande')
        }}
      >
        grande
      </button>
    </div>
  )
}

function mount(store = createMemoryStore()) {
  return {
    store,
    ...render(
      <ThemeProvider store={store}>
        <TextSizeProvider store={store}>
          <Probe />
        </TextSizeProvider>
      </ThemeProvider>,
    ),
  }
}

describe('preferences store', () => {
  it('validates stored values', () => {
    const store = createMemoryStore({ theme: 'rosa' })
    expect(readChoice(store, 'theme', ['claro', 'oscuro'], 'claro')).toBe('claro')
  })

  it('survives a throwing localStorage', () => {
    const spy = vi.spyOn(Storage.prototype, 'getItem').mockImplementation(() => {
      throw new Error('blocked')
    })
    const set = vi.spyOn(Storage.prototype, 'setItem').mockImplementation(() => {
      throw new Error('blocked')
    })
    const store = createLocalStorageStore()
    expect(store.get('x')).toBeNull()
    store.set('x', '1')
    expect(store.get('x')).toBe('1')
    spy.mockRestore()
    set.mockRestore()
  })
})

describe('ThemeProvider and TextSizeProvider', () => {
  it('default to light theme and normal text', () => {
    const { getByTestId } = mount()
    expect(getByTestId('theme')).toHaveTextContent('claro/light')
    expect(getByTestId('size')).toHaveTextContent('normal')
    expect(document.documentElement.dataset['theme']).toBe('light')
    expect(document.documentElement.dataset['textSize']).toBe('normal')
  })

  it('applies and persists the choices', async () => {
    const user = userEvent.setup()
    const { getByText, getByTestId, store } = mount()
    await user.click(getByText('oscuro'))
    await user.click(getByText('grande'))
    expect(document.documentElement.dataset['theme']).toBe('dark')
    expect(document.documentElement.dataset['textSize']).toBe('muy-grande')
    expect(getByTestId('theme')).toHaveTextContent('oscuro/dark')
    expect(store.get('theme')).toBe('oscuro')
    expect(store.get('text-size')).toBe('muy-grande')
  })

  it('restores persisted choices', () => {
    const { getByTestId } = mount(createMemoryStore({ theme: 'oscuro', 'text-size': 'grande' }))
    expect(getByTestId('theme')).toHaveTextContent('oscuro/dark')
    expect(getByTestId('size')).toHaveTextContent('grande')
  })

  it('"sistema" follows prefers-color-scheme live', async () => {
    const user = userEvent.setup()
    setSystemDark(false)
    const { getByText, getByTestId } = mount()
    await user.click(getByText('sistema'))
    expect(getByTestId('theme')).toHaveTextContent('sistema/light')
    act(() => {
      setSystemDark(true)
    })
    expect(getByTestId('theme')).toHaveTextContent('sistema/dark')
    expect(document.documentElement.dataset['theme']).toBe('dark')
    act(() => {
      setSystemDark(false)
    })
  })

  it('scales the root font size 100 / 112.5 / 125 %', () => {
    expect(textSizeScale).toEqual({ normal: 1, grande: 1.125, 'muy-grande': 1.25 })
  })
})
