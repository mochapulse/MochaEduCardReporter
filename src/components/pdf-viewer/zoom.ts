/** Zoom model of the viewer. Pure functions, no DOM. */

export const MIN_ZOOM = 0.5
export const MAX_ZOOM = 3

/** The zoom levels reachable with "Acercar" / "Alejar". */
export const ZOOM_STEPS: readonly number[] = [
  0.5, 0.6, 0.75, 0.9, 1, 1.1, 1.25, 1.5, 1.75, 2, 2.5, 3,
]

/** "Fit width" may go slightly below the manual minimum in a very narrow panel. */
const MIN_FIT_ZOOM = 0.25

export type ZoomMode =
  { readonly kind: 'fit' } | { readonly kind: 'custom'; readonly scale: number }

export const FIT_WIDTH: ZoomMode = { kind: 'fit' }

const EPSILON = 0.005

/** Next step above `scale`, or the maximum. */
export function zoomInFrom(scale: number): number {
  return ZOOM_STEPS.find((step) => step > scale + EPSILON) ?? MAX_ZOOM
}

/** Next step below `scale`, or the minimum. */
export function zoomOutFrom(scale: number): number {
  return [...ZOOM_STEPS].reverse().find((step) => step < scale - EPSILON) ?? MIN_ZOOM
}

/**
 * Scale that makes the widest page fill `availableWidth` CSS pixels.
 * Returns 1 while the size is unknown (e.g. before the first measurement).
 */
export function fitWidthScale(availableWidth: number, pageWidth: number): number {
  if (!(availableWidth > 0) || !(pageWidth > 0)) return 1
  return Math.min(MAX_ZOOM, Math.max(MIN_FIT_ZOOM, availableWidth / pageWidth))
}

export function toPercent(scale: number): number {
  return Math.round(scale * 100)
}
