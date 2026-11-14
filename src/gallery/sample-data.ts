/** Sample student rows for the gallery (fixture data, not UI text). */
export interface Student {
  id: string
  number: number
  name: string
  code: string
  absences: number
  finalGrade: string
}

export const students: readonly Student[] = [
  {
    id: '1',
    number: 1,
    name: 'ÁLVAREZ, Camila',
    code: '2025-0101',
    absences: 0,
    finalGrade: '4.5',
  },
  { id: '2', number: 2, name: 'GÓMEZ, Andrés', code: '2025-0102', absences: 2, finalGrade: '3.8' },
  { id: '3', number: 3, name: 'MARTÍNEZ, Ana', code: '2025-0103', absences: 1, finalGrade: '4.1' },
  { id: '4', number: 4, name: 'PÉREZ, Julián', code: '2025-0104', absences: 4, finalGrade: '3.2' },
  {
    id: '5',
    number: 5,
    name: 'RODRÍGUEZ, Lucía',
    code: '2025-0105',
    absences: 0,
    finalGrade: '4.9',
  },
]
