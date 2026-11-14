/** WCAG 2.x contrast helpers. */

function channel(value: number): number {
  const v = value / 255
  return v <= 0.03928 ? v / 12.92 : ((v + 0.055) / 1.055) ** 2.4
}

export function relativeLuminance(hex: string): number {
  const match = /^#([0-9a-f]{2})([0-9a-f]{2})([0-9a-f]{2})$/i.exec(hex)
  if (!match) throw new Error(`Invalid hex color: ${hex}`)
  const [, r, g, b] = match
  return (
    0.2126 * channel(parseInt(r ?? '0', 16)) +
    0.7152 * channel(parseInt(g ?? '0', 16)) +
    0.0722 * channel(parseInt(b ?? '0', 16))
  )
}

export function contrastRatio(foreground: string, background: string): number {
  const a = relativeLuminance(foreground)
  const b = relativeLuminance(background)
  return (Math.max(a, b) + 0.05) / (Math.min(a, b) + 0.05)
}
