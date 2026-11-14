import type { StudentView } from '@/lib/api'
import { studentKey, type StudentKey } from '@/features/assistant/state'

export interface StudentRow {
  readonly key: StudentKey
  readonly student: StudentView
}

export function toRows(students: readonly StudentView[]): StudentRow[] {
  return students.map((student, index) => ({ key: studentKey(student, index), student }))
}

function fold(text: string): string {
  return text
    .normalize('NFD')
    .replace(/\p{Diacritic}/gu, '')
    .toLowerCase()
}

/** Students whose name or code contains the query (accents and case ignored). */
export function filterRows(rows: readonly StudentRow[], query: string): StudentRow[] {
  const needle = fold(query.trim())
  if (needle === '') return [...rows]
  return rows.filter(
    ({ student }) => fold(student.name).includes(needle) || fold(student.code).includes(needle),
  )
}

/**
 * Selection after the table reports a change for the visible rows: rows hidden
 * by the search keep their state.
 */
export function mergeVisibleSelection(
  all: readonly StudentRow[],
  visible: readonly StudentRow[],
  previous: ReadonlySet<StudentKey>,
  nextVisible: ReadonlySet<StudentKey>,
): ReadonlySet<StudentKey> {
  const visibleKeys = new Set(visible.map((row) => row.key))
  const next = new Set<StudentKey>()
  for (const row of all) {
    const keep = visibleKeys.has(row.key) ? nextVisible.has(row.key) : previous.has(row.key)
    if (keep) next.add(row.key)
  }
  return next
}

/** The first selected student that has a number (the one "Ver ejemplo" uses). */
export function firstPreviewable(
  rows: readonly StudentRow[],
  selected: ReadonlySet<StudentKey>,
): StudentRow | null {
  return rows.find((row) => selected.has(row.key) && row.student.number !== null) ?? null
}
