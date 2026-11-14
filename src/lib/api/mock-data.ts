import type { GradebookView, MetadataView, StudentView } from './types'

/** Fixture data for the mock API: names and numbers only, no UI text. */
const FAMILY_NAMES = [
  'ÁLVAREZ',
  'BERNAL',
  'CASTRO',
  'DÍAZ',
  'ESPINOSA',
  'FLÓREZ',
  'GÓMEZ',
  'HERRERA',
  'IBÁÑEZ',
  'JIMÉNEZ',
  'KLINGER',
  'LÓPEZ',
  'MARTÍNEZ',
  'NIETO',
  'OSPINA',
  'PÉREZ',
  'QUINTERO',
  'RODRÍGUEZ',
  'SANABRIA',
  'TORRES',
  'URIBE',
  'VARGAS',
  'WILCHES',
  'YEPES',
  'ZAPATA',
  'ARIAS',
  'BELTRÁN',
  'CORTÉS',
  'DUARTE',
  'ESCOBAR',
]
const GIVEN_NAMES = [
  'Camila',
  'Andrés',
  'Ana',
  'Julián',
  'Lucía',
  'Mateo',
  'Valentina',
  'Sebastián',
  'Isabella',
  'Santiago',
  'Daniela',
  'Nicolás',
  'Sofía',
  'Felipe',
  'Mariana',
]

export function buildStudents(count: number, seed = 0): StudentView[] {
  return Array.from({ length: count }, (_, index) => {
    const number = index + 1
    const family = FAMILY_NAMES[(index + seed) % FAMILY_NAMES.length] ?? 'APELLIDO'
    const given = GIVEN_NAMES[(index * 7 + seed) % GIVEN_NAMES.length] ?? 'Nombre'
    const grade = 2.5 + ((index * 37 + seed * 11) % 25) / 10
    return {
      number,
      name: `${family}, ${given}`,
      code: `2025${String(1000 + number + seed * 100)}`,
      absences: String((index * 3) % 5),
      definitive_grade: grade.toFixed(1),
      first_twenty_grades: [],
      component_grades: [],
      weighted_components: [],
    }
  })
}

export interface MockSheetSpec {
  readonly name: string
  readonly students: number
  readonly metadata: MetadataView
  /** Extra English parser warnings for this sheet. */
  readonly warnings?: readonly string[]
}

export function suggestedFolderName(metadata: MetadataView): string {
  return `${metadata.subject} – Grupo ${metadata.group} – ${metadata.period}`
}

export function buildGradebook(
  id: string,
  path: string,
  spec: MockSheetSpec,
  isExcel: boolean,
  seed: number,
): GradebookView {
  const fileName = path.split(/[\\/]/).pop() ?? path
  return {
    id,
    source_path: path,
    source_file_name: fileName,
    sheet: isExcel ? spec.name : null,
    metadata: spec.metadata,
    students: buildStudents(spec.students, seed),
    warnings: [...(spec.warnings ?? [])],
    suggested_folder_name: suggestedFolderName(spec.metadata),
  }
}

export const FRENCH_IV_MORNING: MetadataView = {
  university: 'Universidad Nacional',
  program: 'Licenciatura en Lenguas',
  subject: 'Francés IV',
  group: '2',
  professor: 'Mariela Duarte',
  period: '2025-2',
}

export const FRENCH_IV_EVENING: MetadataView = { ...FRENCH_IV_MORNING, group: '5' }
export const FRENCH_I: MetadataView = { ...FRENCH_IV_MORNING, subject: 'Francés I', group: '1' }
export const EMPTY_METADATA: MetadataView = {
  university: '',
  program: '',
  subject: '',
  group: '',
  professor: '',
  period: '',
}
