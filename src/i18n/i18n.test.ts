import { describe, expect, it } from 'vitest'
import { errorCodes, errorMessages, getErrorMessage } from './errors'
import { t } from './es'

describe('t()', () => {
  it('interpolates parameters', () => {
    expect(t('stepper.progress', { current: 2, total: 3 })).toBe('Paso 2 de 3')
    expect(t('progress.current', { done: 12, total: 27, name: 'MARTÍNEZ, Ana' })).toBe(
      '12 de 27 — MARTÍNEZ, Ana',
    )
  })

  it('uses Spanish plural forms', () => {
    expect(t('home.history.count', { count: 1 })).toBe('1 boletín')
    expect(t('home.history.count', { count: 27 })).toBe('27 boletines')
    expect(t('home.history.count', { count: 0 })).toBe('0 boletines')
    expect(t('save.generate', { count: 27 })).toBe('Generar 27 boletines')
  })

  it('requires a numeric count for plural entries', () => {
    expect(() => t('save.generate')).toThrow(/count/)
  })

  it('keeps unknown placeholders visible instead of failing', () => {
    expect(t('stepper.progress', { current: 1 })).toBe('Paso 1 de {total}')
  })
})

describe('error messages', () => {
  it('covers every error code of the plan', () => {
    expect([...errorCodes]).toEqual([
      'FILE_NOT_FOUND',
      'FILE_LOCKED',
      'UNSUPPORTED_FORMAT',
      'NO_GRADEBOOK_FOUND',
      'EMPTY_SHEET',
      'OUTPUT_NOT_WRITABLE',
      'PDF_FAILED',
      'CANCELLED',
      'INTERNAL',
    ])
    expect(Object.keys(errorMessages).sort()).toEqual([...errorCodes].sort())
  })

  it('gives every code a title, what happened and what to do', () => {
    for (const code of errorCodes) {
      const message = errorMessages[code]
      expect(message.title.length).toBeGreaterThan(5)
      expect(message.whatHappened.length).toBeGreaterThan(10)
      expect(message.whatToDo.length).toBeGreaterThan(10)
    }
  })

  it('avoids technical jargon and raw codes', () => {
    const jargon =
      /\b(error|exception|stack|null|undefined|pdf|path|buffer|sheet|workspace)\b|[A-Z]+_[A-Z_]+/i
    for (const code of errorCodes) {
      const { title, whatHappened, whatToDo } = errorMessages[code]
      for (const text of [title, whatHappened, whatToDo]) {
        // "PDF" is allowed only as part of the app's own vocabulary; keep the list honest.
        expect(text.replace(/\bPDF\b/g, '')).not.toMatch(jargon)
      }
    }
  })

  it('maps FILE_LOCKED to the Excel advice', () => {
    expect(errorMessages.FILE_LOCKED.whatToDo).toBe(
      'Cierra el archivo en Excel e inténtalo de nuevo.',
    )
  })

  it('falls back to INTERNAL for unknown codes', () => {
    expect(getErrorMessage('SOMETHING_NEW')).toBe(errorMessages.INTERNAL)
  })
})
