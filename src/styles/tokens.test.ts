import { readFileSync } from 'node:fs'
import path from 'node:path'
import { describe, expect, it } from 'vitest'
import { BASE_FONT_PX, MIN_FONT_PX, colorTokenNames, colorTokens, typeScaleRem } from './tokens'

const css = readFileSync(path.join(import.meta.dirname, 'tokens.css'), 'utf8')

function block(selector: string): string {
  const start = css.indexOf(selector)
  expect(start, `selector ${selector} in tokens.css`).toBeGreaterThanOrEqual(0)
  return css.slice(start, css.indexOf('}', start))
}

describe('tokens.css mirrors tokens.ts', () => {
  const blocks = {
    light: block(":root[data-theme='light']"),
    dark: block(":root[data-theme='dark']"),
  }
  for (const theme of ['light', 'dark'] as const) {
    it.each(colorTokenNames)(`${theme}: --%s`, (name) => {
      const match = new RegExp(`--${name}:\\s*(#[0-9a-fA-F]{6})\\s*;`).exec(blocks[theme])
      expect(match?.[1]?.toUpperCase()).toBe(colorTokens[theme][name].toUpperCase())
    })
  }
})

describe('type scale', () => {
  it('never renders below 14 px at normal text size', () => {
    for (const rem of Object.values(typeScaleRem)) {
      expect(rem * BASE_FONT_PX).toBeGreaterThanOrEqual(MIN_FONT_PX)
    }
  })

  it('matches the CSS variables', () => {
    for (const [name, rem] of Object.entries(typeScaleRem)) {
      expect(css).toContain(`--font-size-${name}: ${String(rem)}rem;`)
    }
  })
})

describe('motion', () => {
  it('collapses durations under prefers-reduced-motion', () => {
    const reduced = css.slice(css.indexOf('prefers-reduced-motion'))
    expect(reduced).toMatch(/--motion-fast:\s*0\.01ms/)
    expect(reduced).toMatch(/--motion-base:\s*0\.01ms/)
  })
})
