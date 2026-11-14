import { describe, expect, it } from 'vitest'
import { contrastRatio } from './contrast'
import { colorTokens, type ColorTokenName, type ThemeName } from './tokens'

interface Pair {
  fg: ColorTokenName
  bg: ColorTokenName
  min: number
  use: string
}

const pairs: Pair[] = []
const add = (fgs: ColorTokenName[], bgs: ColorTokenName[], min: number, use: string) => {
  for (const fg of fgs) for (const bg of bgs) pairs.push({ fg, bg, min, use })
}

// Body text: plan section 5 asks for >= 7:1.
add(['text'], ['background', 'surface', 'surface-muted'], 7, 'body text')
// Everything else >= 4.5:1.
add(['text-muted'], ['background', 'surface', 'surface-muted'], 4.5, 'secondary text')
add(['on-primary'], ['primary', 'primary-hover'], 4.5, 'text on the primary button')
add(['success', 'warning', 'danger'], ['surface', 'background'], 4.5, 'status icon/text')
add(['primary', 'accent'], ['surface', 'background'], 4.5, 'links, outlined buttons, icons')
// Outlined/ghost buttons sit on surface-muted when hovered.
add(['primary'], ['surface-muted'], 4.5, 'hovered outline button')
add(['danger'], ['surface-muted'], 4.5, 'hovered danger button')
// Tooltip: inverted text on text-colored background.
add(['background'], ['text'], 7, 'tooltip text')
// Keyboard focus outline: non-text contrast >= 3:1 against what it touches.
add(['focus-ring'], ['background', 'surface', 'surface-muted'], 3, 'focus ring')

describe.each(['light', 'dark'] as ThemeName[])('contrast (%s theme)', (theme) => {
  const palette = colorTokens[theme]
  it.each(pairs)('$fg on $bg ($use) >= $min:1', ({ fg, bg, min }) => {
    const ratio = contrastRatio(palette[fg], palette[bg])
    expect(
      ratio,
      `${theme}: ${fg} ${palette[fg]} on ${bg} ${palette[bg]} = ${ratio.toFixed(2)}`,
    ).toBeGreaterThanOrEqual(min)
  })
})
