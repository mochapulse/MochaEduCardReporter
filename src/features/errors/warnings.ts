import { t } from '@/i18n/es'

export interface TranslatedWarning {
  /** Plain Spanish text. */
  readonly text: string
  /** The original English engine text, for "Ver detalles". */
  readonly original: string
}

/**
 * Counts the students named in the engine's numbering-correction warning:
 * `Student enumeration corrected from row order: A: 8 -> 7; B: 7 -> 8; and 3 more`.
 */
function countCorrected(listing: string): number {
  const more = /;?\s*and (\d+) more$/.exec(listing)
  const listed = (more ? listing.slice(0, more.index) : listing)
    .split(';')
    .filter((part) => part.trim() !== '').length
  return listed + (more?.[1] ? Number(more[1]) : 0)
}

/**
 * Turns an English engine warning into plain Spanish. Known patterns get their
 * own wording; anything else gets a generic notice (the original stays available).
 */
export function translateWarning(original: string): TranslatedWarning {
  const numbering = /^Student enumeration corrected from row order:\s*(.*)$/s.exec(original)
  if (numbering) {
    return {
      text: t('review.renumbered', { count: countCorrected(numbering[1] ?? '') }),
      original,
    }
  }
  return { text: t('review.warningGeneric'), original }
}
