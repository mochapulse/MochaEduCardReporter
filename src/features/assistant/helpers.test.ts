import { describe, expect, it } from 'vitest'
import { translateWarning } from '@/features/errors/warnings'
import { filterRows, mergeVisibleSelection, toRows } from '@/features/review/students'
import { buildStudents } from '@/lib/api/mock-data'
import { formatDisplayDate, isValidIsoDate, todayIso } from './dates'
import { baseName, isSupportedGradebook, joinPath, studentFromFileName } from './paths'

describe('dates', () => {
  it('writes the local date as YYYY-MM-DD and shows it as DD/MM/YYYY', () => {
    expect(todayIso(new Date(2026, 10, 4))).toBe('2026-11-04')
    expect(formatDisplayDate('2026-11-04')).toBe('04/11/2026')
    expect(formatDisplayDate('2026-11-04T10:00:00')).toBe('04/11/2026')
  })
  it('validates real calendar dates only', () => {
    expect(isValidIsoDate('2026-02-28')).toBe(true)
    expect(isValidIsoDate('2026-02-30')).toBe(false)
    expect(isValidIsoDate('')).toBe(false)
    expect(isValidIsoDate('14/11/2026')).toBe(false)
  })
})

describe('paths', () => {
  it('joins with the separator the folder uses', () => {
    expect(joinPath('/home/a/', 'x')).toBe('/home/a/x')
    expect(joinPath('C:\\Users\\a', 'x')).toBe('C:\\Users\\a\\x')
  })
  it('knows base names, supported files and student file names', () => {
    expect(baseName('C:\\a\\b.xlsx')).toBe('b.xlsx')
    expect(isSupportedGradebook('/a/NOTAS.XLS')).toBe(true)
    expect(isSupportedGradebook('/a/notas.pdf')).toBe(false)
    expect(studentFromFileName('/a/07_MARTINEZ_Ana.pdf')).toEqual({
      number: 7,
      name: 'MARTINEZ Ana',
    })
    expect(studentFromFileName('/a/sin-numero.pdf').number).toBeNull()
  })
})

describe('engine warnings in Spanish', () => {
  it('translates the numbering correction with the count', () => {
    const listed = translateWarning(
      'Student enumeration corrected from row order: A: 8 -> 7; B: 7 -> 8; C: 12 -> 11',
    )
    expect(listed.text).toBe(
      'La numeración de 3 estudiantes estaba incorrecta en el archivo y se corrigió automáticamente.',
    )
    expect(listed.original).toContain('Student enumeration')
  })
  it('counts the "and N more" tail and uses the singular', () => {
    const many = translateWarning(
      'Student enumeration corrected from row order: A: 1 -> 2; B: 2 -> 3; C: 3 -> 4; D: 4 -> 5; E: 5 -> 6; and 7 more',
    )
    expect(many.text).toContain('12 estudiantes')
    const one = translateWarning('Student enumeration corrected from row order: A: 1 -> 2')
    expect(one.text).toContain('1 estudiante estaba')
  })
  it('falls back to a generic notice for anything else', () => {
    const other = translateWarning('Something odd happened')
    expect(other.text).toMatch(/inusual/)
    expect(other.original).toBe('Something odd happened')
  })
})

describe('student table helpers', () => {
  const rows = toRows(buildStudents(27))
  it('filters by name or code ignoring accents and case', () => {
    expect(filterRows(rows, 'alvarez').map((r) => r.student.name)).toEqual(['ÁLVAREZ, Camila'])
    expect(filterRows(rows, '  ').length).toBe(27)
    expect(filterRows(rows, rows[3]?.student.code ?? '').length).toBe(1)
    expect(filterRows(rows, 'zzzz')).toEqual([])
  })
  it('keeps the selection of rows hidden by the search', () => {
    const visible = filterRows(rows, 'alvarez')
    const previous = new Set(['2', '3', '1'])
    const next = mergeVisibleSelection(rows, visible, previous, new Set())
    expect([...next]).toEqual(['2', '3'])
  })
})
