/** Error codes returned by the backend (PLAN_REDO_CODE.md section 8). */
export const errorCodes = [
  'FILE_NOT_FOUND',
  'FILE_LOCKED',
  'UNSUPPORTED_FORMAT',
  'NO_GRADEBOOK_FOUND',
  'EMPTY_SHEET',
  'OUTPUT_NOT_WRITABLE',
  'PDF_FAILED',
  'CANCELLED',
  'INTERNAL',
] as const

export type ErrorCode = (typeof errorCodes)[number]

export interface ErrorMessage {
  /** Short heading. */
  readonly title: string
  /** One plain sentence: what happened. */
  readonly whatHappened: string
  /** One plain sentence: what the person can do next. */
  readonly whatToDo: string
}

/** Plain Spanish, no jargon. Technical detail is shown separately, behind "Ver detalles". */
export const errorMessages: Readonly<Record<ErrorCode, ErrorMessage>> = {
  FILE_NOT_FOUND: {
    title: 'No se encontró el archivo',
    whatHappened: 'El archivo ya no está en el lugar donde estaba. Pudo haberse movido o borrado.',
    whatToDo: 'Elegir el archivo de notas de nuevo.',
  },
  FILE_LOCKED: {
    title: 'El archivo está abierto en otro programa',
    whatHappened: 'No se puede leer el archivo porque otro programa lo está usando.',
    whatToDo: 'Cierra el archivo en Excel e inténtalo de nuevo.',
  },
  UNSUPPORTED_FORMAT: {
    title: 'Este tipo de archivo no se puede usar',
    whatHappened: 'El archivo elegido no es una hoja de cálculo de Excel ni un archivo CSV.',
    whatToDo: 'Elegir un archivo de Excel (.xlsx o .xls) o un archivo CSV.',
  },
  NO_GRADEBOOK_FOUND: {
    title: 'No se encontró una hoja de notas',
    whatHappened:
      'El archivo se abrió, pero ninguna hoja tiene el formato de un cuaderno de notas.',
    whatToDo: 'Revisar que sea el archivo correcto, o elegir otro archivo de notas.',
  },
  EMPTY_SHEET: {
    title: 'La hoja está vacía',
    whatHappened: 'La hoja elegida no tiene estudiantes ni notas.',
    whatToDo: 'Elegir otra hoja del archivo, o completar las notas y volver a intentarlo.',
  },
  OUTPUT_NOT_WRITABLE: {
    title: 'No se pudo guardar en esa carpeta',
    whatHappened: 'El programa no tiene permiso para guardar archivos en la carpeta elegida.',
    whatToDo: 'Elegir otra carpeta, por ejemplo Documentos.',
  },
  PDF_FAILED: {
    title: 'No se pudo crear un boletín',
    whatHappened: 'Hubo un problema al crear el boletín de uno de los estudiantes.',
    whatToDo:
      'Pulsar «Reintentar». Si vuelve a fallar, copiar los detalles y enviarlos a quien da soporte.',
  },
  CANCELLED: {
    title: 'Se canceló la operación',
    whatHappened:
      'Se detuvo la generación porque lo pediste. Los boletines ya creados se conservan.',
    whatToDo: 'Volver a generar cuando quieras.',
  },
  INTERNAL: {
    title: 'Algo salió mal',
    whatHappened: 'El programa tuvo un problema inesperado. Tus archivos no se modificaron.',
    whatToDo:
      'Inténtalo de nuevo. Si sigue pasando, copiar los detalles y enviarlos a quien da soporte.',
  },
}

/** Fallback to INTERNAL for any code the frontend does not know yet. */
export function getErrorMessage(code: string): ErrorMessage {
  return (
    (errorMessages as Readonly<Record<string, ErrorMessage | undefined>>)[code] ??
    errorMessages.INTERNAL
  )
}
