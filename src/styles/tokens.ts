/**
 * Typed mirror of the color tokens defined in `tokens.css`.
 * Source of truth for values: PLAN_REDO_CODE.md section 5.1.
 * A test asserts that this file and `tokens.css` never drift apart.
 */

export const colorTokenNames = [
  'primary',
  'primary-hover',
  'on-primary',
  'accent',
  'background',
  'surface',
  'surface-muted',
  'border',
  'text',
  'text-muted',
  'focus-ring',
  'success',
  'warning',
  'danger',
] as const

export type ColorTokenName = (typeof colorTokenNames)[number]
export type ThemeName = 'light' | 'dark'
export type ColorPalette = Record<ColorTokenName, string>

export const colorTokens: Record<ThemeName, ColorPalette> = {
  light: {
    primary: '#0F766E',
    'primary-hover': '#115E59',
    'on-primary': '#FFFFFF',
    accent: '#0E7490',
    background: '#F3FAF9',
    surface: '#FFFFFF',
    'surface-muted': '#E3F2F1',
    border: '#B9D8D6',
    text: '#0F2A2E',
    'text-muted': '#44646A',
    'focus-ring': '#0E7490',
    success: '#15803D',
    warning: '#B45309',
    danger: '#B91C1C',
  },
  dark: {
    primary: '#2DD4BF',
    'primary-hover': '#5EEAD4',
    'on-primary': '#042F2E',
    accent: '#22D3EE',
    background: '#0B1F22',
    surface: '#12292D',
    'surface-muted': '#183539',
    border: '#2A4D52',
    text: '#E8F5F4',
    'text-muted': '#9FC2C1',
    'focus-ring': '#67E8F9',
    success: '#4ADE80',
    warning: '#FBBF24',
    danger: '#F87171',
  },
}

/** Root font size at 100 % text size, in px. All sizes are rem-based. */
export const BASE_FONT_PX = 17

/** Type scale in rem (multiply by BASE_FONT_PX for px at "normal" size). */
export const typeScaleRem = {
  xs: 0.85,
  sm: 0.9,
  base: 1,
  lg: 1.2,
  xl: 1.45,
  '2xl': 1.8,
  '3xl': 2.2,
} as const

/** Minimum allowed rendered text size in px (plan section 5). */
export const MIN_FONT_PX = 14

/** Minimum hit area for any clickable control, in px (plan section 5). */
export const MIN_HIT_PX = 44
