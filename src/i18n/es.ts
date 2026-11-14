/**
 * The single Spanish dictionary of the app. Every user-visible string lives here.
 * Style: neutral, plain, imperative ("Elegir archivo de notas"), no jargon.
 * Values may contain `{name}` placeholders; plural entries are `{ one, other }`
 * and need a `count` parameter (see `t()`).
 */

export interface PluralForms {
  readonly one: string
  readonly other: string
}

type Leaf = string | PluralForms
interface Dict {
  readonly [key: string]: Leaf | Dict
}

export const es = {
  app: {
    name: 'Generador de boletines',
    version: 'Versión {version}',
  },

  common: {
    continue: 'Continuar',
    back: 'Atrás',
    cancel: 'Cancelar',
    accept: 'Aceptar',
    close: 'Cerrar',
    clear: 'Borrar',
    search: 'Buscar',
    change: 'Cambiar',
    retry: 'Reintentar',
    copy: 'Copiar',
    copied: 'Copiado',
    seeDetails: 'Ver detalles',
    hideDetails: 'Ocultar detalles',
    settings: 'Ajustes',
    help: 'Ayuda',
    optional: 'Opcional',
    emptyValue: '—',
    group: 'Grupo {group}',
    yes: 'Sí',
    no: 'No',
  },

  windowTitle: {
    format: '{page} — {app}',
  },

  picker: {
    chooseFile: 'Elegir archivo de notas',
    filterName: 'Archivos de notas',
    chooseFolder: 'Elegir la carpeta donde guardar los boletines',
  },

  stepper: {
    label: 'Pasos del asistente',
    progress: 'Paso {current} de {total}',
    steps: {
      review: 'Revisar',
      save: 'Guardar',
      generate: 'Generar',
    },
    state: {
      done: 'completado',
      current: 'paso actual',
      upcoming: 'pendiente',
    },
  },

  notice: {
    info: 'Información',
    success: 'Listo',
    warning: 'Atención',
    danger: 'Error',
  },

  dialog: {
    defaultConfirm: 'Sí, continuar',
    defaultCancel: 'No, volver',
  },

  table: {
    selectAll: 'Seleccionar todos',
    selectNone: 'Quitar la selección',
    selectRow: 'Seleccionar a {name}',
    selectedCount: {
      one: '{count} seleccionado de {total}',
      other: '{count} seleccionados de {total}',
    },
    scrollRegion: 'Tabla desplazable',
  },

  search: {
    clear: 'Borrar búsqueda',
    noResults: 'No se encontró nada con «{query}».',
    noResultsHint: 'Revisar la ortografía o borrar la búsqueda.',
  },

  path: {
    change: 'Cambiar carpeta…',
    fullPathHint: 'Ruta completa: {path}',
  },

  dropZone: {
    title: 'Arrastra aquí tu archivo de notas',
    dragOver: 'Suelta el archivo aquí',
    or: 'o también puedes',
    button: 'Elegir archivo de notas',
    formats: 'Archivos admitidos: {formats}',
    rejected: 'Ese archivo no se puede usar. Elegir un archivo de Excel (.xlsx o .xls) o CSV.',
  },

  progress: {
    current: '{done} de {total} — {name}',
    finishing: '{done} de {total}',
    cancel: 'Cancelar',
  },

  empty: {
    noStudents: 'No hay estudiantes para mostrar.',
    noRecents: 'Todavía no hay archivos recientes.',
    noHistory: 'Todavía no se han generado boletines.',
  },

  home: {
    title: 'Inicio',
    heading: 'Crear boletines de notas',
    subtitle: 'Elegir el archivo de notas para crear los boletines.',
    opening: 'Abriendo el archivo…',
    recents: {
      title: 'Archivos recientes',
      open: 'Abrir {name}',
      missing: 'Este archivo ya no está en su lugar. Elegir el archivo de nuevo.',
      openedOn: 'Abierto el {date}',
    },
    history: {
      title: 'Boletines anteriores',
      count: { one: '{count} boletín', other: '{count} boletines' },
      generatedOn: 'Generados el {date}',
      open: 'Ver boletines',
      remove: 'Quitar de la lista',
      removeLabel: 'Quitar de la lista: {name}',
      folderMissing: 'La carpeta de estos boletines ya no está en su lugar.',
      untitled: 'Boletines sin título',
      confirmRemove: {
        title: '¿Quitar de la lista?',
        description: 'Solo se quita de esta lista. Los archivos PDF no se borran.',
        confirm: 'Sí, quitar',
        keep: 'No, dejarlo',
      },
    },
  },

  review: {
    title: 'Paso 1 — Revisar',
    chooseSheet: 'Elegir la hoja con las notas',
    sheetStudents: { one: '{count} estudiante', other: '{count} estudiantes' },
    useSheet: 'Usar esta hoja',
    opening: 'Leyendo la hoja…',
    summary: {
      title: 'Resumen del archivo',
      sheet: 'Hoja',
      subject: 'Materia',
      group: 'Grupo',
      teacher: 'Profesor',
      period: 'Periodo',
    },
    students: {
      title: 'Estudiantes',
      number: 'N.º',
      name: 'Nombre',
      code: 'Código',
      absences: 'Faltas',
      finalGrade: 'Nota final',
      selectAllNone: 'Seleccionar todos / ninguno',
      searchLabel: 'Buscar estudiante',
      searchPlaceholder: 'Escribir un nombre o código',
      caption: 'Estudiantes del archivo',
      selected: {
        one: '{count} estudiante seleccionado de {total}',
        other: '{count} estudiantes seleccionados de {total}',
      },
    },
    warningsTitle: 'Avisos sobre el archivo',
    warningGeneric:
      'Se encontró algo inusual al leer el archivo. Revisar los datos antes de continuar.',
    preview: 'Ver ejemplo',
    previewTitle: 'Ejemplo de boletín',
    previewPreparing: 'Creando el ejemplo…',
    previewFor: 'Ejemplo con los datos de {name}.',
    renumbered: {
      one: 'La numeración de {count} estudiante estaba incorrecta en el archivo y se corrigió automáticamente.',
      other:
        'La numeración de {count} estudiantes estaba incorrecta en el archivo y se corrigió automáticamente.',
    },
    previewHint: 'Crea un solo boletín de prueba para ver cómo queda.',
    nothingSelected: 'Elegir al menos un estudiante para continuar.',
  },

  save: {
    title: 'Paso 2 — Guardar',
    savingIn: 'Los boletines se guardarán en:',
    changeFolder: 'Cambiar carpeta…',
    rememberBase: 'Recordar esta carpeta base',
    dateLabel: 'Fecha del boletín',
    dateToday: 'Hoy',
    summary: {
      one: 'Se creará {count} boletín en {path}.',
      other: 'Se crearán {count} boletines en {path}.',
    },
    generate: {
      one: 'Generar {count} boletín',
      other: 'Generar {count} boletines',
    },
    disabledReason: 'Para generar, primero elegir una carpeta donde guardar.',
    dateHelp: 'Esta fecha se imprime en cada boletín.',
    datePrinted: 'Se imprimirá como {date}.',
    dateInvalid: 'Para generar, escribir una fecha válida.',
    checking: 'Revisando la carpeta…',
    notWritable: 'No se puede guardar en esa carpeta. Elegir otra carpeta.',
    chooseExisting: 'Para generar, elegir primero qué hacer con los boletines que ya existen.',
    folderExists: {
      title: 'Ya hay boletines en esta carpeta',
      found: {
        one: 'La carpeta ya tiene {count} boletín.',
        other: 'La carpeta ya tiene {count} boletines.',
      },
      description: 'Elegir qué hacer antes de guardar. No se escribirá nada hasta que elijas.',
      choose: '¿Qué hacer con ellos?',
      replaceHint: 'Los boletines nuevos reemplazan a los que ya están.',
      newFolderHint: 'Se guardan aparte, en la carpeta «{name}».',
      replace: 'Reemplazar',
      newFolder: 'Guardar en una carpeta nueva',
    },
  },

  generating: {
    title: 'Generando boletines',
    inProgress: 'Se están creando los boletines. No cierres la ventana.',
    preparing: 'Preparando…',
    cancelling: 'Cancelando… se termina el boletín que está en curso.',
    failedSoFar: {
      one: 'No se pudo crear {count} boletín hasta ahora.',
      other: 'No se pudieron crear {count} boletines hasta ahora.',
    },
    cancel: 'Cancelar',
    confirmCancel: {
      title: '¿Cancelar la generación?',
      description: 'Los boletines que ya se crearon se conservan. Los demás no se crearán.',
      confirm: 'Sí, cancelar',
      keep: 'No, seguir generando',
    },
    cancelled: {
      one: 'Se canceló. Se conserva {count} boletín ya creado.',
      other: 'Se canceló. Se conservan {count} boletines ya creados.',
    },
    failures: {
      title: {
        one: 'No se pudo crear {count} boletín',
        other: 'No se pudieron crear {count} boletines',
      },
      description: 'Los demás boletines se crearon bien. Se puede reintentar los que fallaron.',
      retry: 'Reintentar los que fallaron',
      student: '{name}: {reason}',
    },
  },

  results: {
    title: 'Resultados',
    done: {
      one: 'Listo. Se creó {count} boletín.',
      other: 'Listo. Se crearon {count} boletines.',
    },
    historyTitle: 'Boletines anteriores',
    listTitle: 'Boletines creados',
    noneCreated: 'No se creó ningún boletín.',
    noneCreatedHint: 'Volver al paso anterior para intentarlo de nuevo.',
    chooseOne: 'Elegir un boletín de la lista para verlo.',
    noMatches: 'Ningún boletín coincide con la búsqueda.',
    viewerFor: 'Boletín de {name}',
    searchLabel: 'Buscar boletín',
    viewer: {
      zoomIn: 'Acercar',
      zoomOut: 'Alejar',
      fitWidth: 'Ajustar al ancho',
      previousPage: 'Página anterior',
      nextPage: 'Página siguiente',
      page: 'Página {page} de {total}',
      previousStudent: 'Estudiante anterior',
      nextStudent: 'Estudiante siguiente',
      loading: 'Cargando el boletín…',
      failed: 'No se pudo mostrar este boletín.',
    },
    actions: {
      openFolder: 'Abrir carpeta',
      openWith: 'Abrir con otro programa',
      print: 'Imprimir',
      generateOthers: 'Generar otros boletines',
      printHint:
        'Se abre el boletín en el programa de PDF del equipo; desde allí se puede imprimir.',
    },
  },

  settings: {
    title: 'Ajustes',
    intro: 'Nada de esto es necesario para usar el programa.',
    baseFolder: 'Carpeta base para guardar',
    textSize: {
      title: 'Tamaño del texto',
      normal: 'Normal',
      large: 'Grande',
      extraLarge: 'Muy grande',
    },
    theme: {
      title: 'Tema',
      light: 'Claro',
      dark: 'Oscuro',
      system: 'Según el sistema',
    },
    updates: {
      title: 'Actualizaciones',
      check: 'Buscar actualizaciones',
      checking: 'Buscando actualizaciones…',
      upToDate: 'Tienes la última versión.',
      available: 'Hay una versión nueva: {version}.',
      install: 'Actualizar ahora',
      installing: 'Instalando la versión {version}…',
      restartHint: 'El programa se reiniciará al terminar. Los boletines ya creados no se tocan.',
    },
    baseFolderHelp: 'Los boletines se guardan en una carpeta nueva dentro de esta.',
    useDefaultFolder: 'Usar la carpeta predeterminada',
    saveFailed: 'No se pudieron guardar los ajustes. Se usarán solo hasta cerrar el programa.',
    logs: 'Registros del programa',
    logsHelp: 'Solo hace falta si quien da soporte lo pide.',
    openLogs: 'Abrir carpeta de registros',
  },

  helpPage: {
    title: 'Ayuda',
    stepsTitle: 'Los tres pasos',
    step1: 'Elegir el archivo de notas y revisar los estudiantes.',
    step2: 'Elegir la carpeta donde guardar los boletines.',
    step3: 'Generar los boletines y revisarlos.',
    formatsTitle: 'Archivos que se pueden usar',
    formats: 'Excel (.xlsx y .xls) y CSV (.csv).',
    notRecognizedTitle: 'Si el archivo no se reconoce',
    notRecognized:
      'Revisar que sea el archivo de notas y que tenga estudiantes y notas. Cerrar el archivo en Excel antes de elegirlo.',
    intro: 'Esta página resume cómo crear los boletines.',
  },

  error: {
    whatHappened: 'Qué pasó',
    whatToDo: 'Qué hacer',
    technicalDetails: 'Detalles técnicos',
    copyDetails: 'Copiar detalles',
  },

  pdfViewer: {
    toolbarLabel: 'Controles del visor',
    pagesLabel: 'Páginas del boletín',
    zoomOut: 'Alejar',
    zoomIn: 'Acercar',
    fitWidth: 'Ajustar al ancho',
    zoomValue: '{percent} %',
    zoomAnnouncement: 'Zoom {percent} %',
    fitWidthAnnouncement: 'Ajustado al ancho, {percent} %',
    page: 'Página {page} de {total}',
    pageLabel: 'Página {page}',
    previousPage: 'Página anterior',
    nextPage: 'Página siguiente',
    previousDocument: 'Anterior',
    nextDocument: 'Siguiente',
    documentsLabel: 'Cambiar de boletín',
    showing: 'Mostrando boletín de {title}',
    loading: 'Cargando el boletín…',
    failed: 'No se pudo mostrar este boletín.',
    failedHelp: 'Pruebe de nuevo. Si sigue sin verse, abra el boletín con otro programa.',
    retry: 'Reintentar',
    technicalDetails: 'Ver detalles técnicos',
    pageFailed: 'No se pudo dibujar esta página.',
  },

  gallery: {
    title: 'Galería de componentes',
    intro: 'Página de pruebas del diseño. Muestra cada componente en cada estado.',
    appearance: 'Apariencia',
    theme: 'Tema',
    textSize: 'Tamaño del texto',
    sections: {
      buttons: 'Botones',
      cards: 'Tarjetas',
      stepper: 'Pasos',
      table: 'Tabla de estudiantes',
      notices: 'Avisos',
      dialog: 'Diálogo de confirmación',
      progress: 'Progreso',
      empty: 'Estado vacío',
      pathField: 'Carpeta',
      search: 'Búsqueda',
      checkbox: 'Casilla',
      dropZone: 'Zona para arrastrar',
      tooltip: 'Sugerencia',
      errors: 'Mensajes de error',
    },
    states: {
      normal: 'Normal',
      disabled: 'Desactivado',
      withIcon: 'Con icono',
      checked: 'Marcada',
      unchecked: 'Sin marcar',
      indeterminate: 'Mixta',
      dragOver: 'Con un archivo encima',
      empty: 'Vacía',
      step: 'En el paso {current}',
    },
    sample: {
      cardTitle: 'Francés IV — Grupo 2',
      cardBody: '27 estudiantes. Hoja «FRANCAIS IV LE MATIN».',
      noticeInfo: 'Este mensaje explica algo sin urgencia.',
      noticeSuccess: 'Los boletines se crearon correctamente.',
      noticeWarning:
        'La numeración de 3 estudiantes estaba incorrecta en el archivo y se corrigió automáticamente.',
      noticeDanger: 'No se pudo guardar en esa carpeta.',
      noticeTitle: 'Título del aviso',
      folder: '/home/profesora/Documentos/Boletines/Francés IV – Grupo 2 – 2025-2',
      chosenFile: 'Se eligió un archivo (demostración).',
      droppedFile: 'Se soltó un archivo (demostración).',
      rejectedFile: 'Se soltó un archivo no admitido (demostración).',
      changeFolderClicked: 'Se pulsó «Cambiar carpeta…» (demostración).',
      tooltipTrigger: 'Pasar el cursor o enfocar aquí',
      tooltipContent: 'Esta sugerencia solo repite lo que ya está escrito.',
      openDialog: 'Abrir el diálogo',
      dialogTitle: '¿Reemplazar los boletines?',
      dialogDescription:
        'Ya hay 27 boletines en esta carpeta. Si continúas, se reemplazarán por los nuevos.',
      dialogConfirm: 'Sí, reemplazar',
      dialogCancel: 'No, volver',
      dialogResult: 'Resultado del diálogo: {result}',
      emptyTitle: 'No hay estudiantes para mostrar',
      emptyDescription: 'Elegir otra hoja del archivo.',
      emptyAction: 'Elegir otra hoja',
      primary: 'Primario',
      secondary: 'Secundario',
      ghost: 'Discreto',
      danger: 'Peligroso',
      checkboxLabel: 'Recordar esta carpeta base',
      progressName: 'MARTÍNEZ, Ana',
      searchLabel: 'Buscar estudiante',
      searchPlaceholder: 'Escribir un nombre',
    },
  },
} as const satisfies Dict

type PathsOf<T, Prefix extends string = ''> = {
  [K in keyof T & string]: T[K] extends string | PluralForms
    ? `${Prefix}${K}`
    : PathsOf<T[K], `${Prefix}${K}.`>
}[keyof T & string]

/** Every valid dictionary key, e.g. `'stepper.progress'`. */
export type MessageKey = PathsOf<typeof es>

export type MessageParams = Readonly<Record<string, string | number>>

const pluralRules = new Intl.PluralRules('es')

function isPluralForms(value: unknown): value is PluralForms {
  return (
    typeof value === 'object' &&
    value !== null &&
    'one' in value &&
    'other' in value &&
    typeof value.one === 'string' &&
    typeof value.other === 'string'
  )
}

function lookup(key: string): string | PluralForms {
  let node: unknown = es
  for (const part of key.split('.')) {
    if (typeof node !== 'object' || node === null || !(part in node)) {
      throw new Error(`Missing i18n key: ${key}`)
    }
    node = (node as Record<string, unknown>)[part]
  }
  if (typeof node === 'string' || isPluralForms(node)) return node
  throw new Error(`i18n key is not a message: ${key}`)
}

function interpolate(template: string, params: MessageParams): string {
  return template.replace(/\{(\w+)\}/g, (placeholder, name: string) => {
    const value = params[name]
    return value === undefined ? placeholder : String(value)
  })
}

/**
 * Looks up a message, picks the Spanish plural form when the entry is a plural
 * (using `params.count`) and replaces `{name}` placeholders.
 *
 * @example t('save.generate', { count: 1 }) // "Generar 1 boletín"
 * @example t('save.generate', { count: 27 }) // "Generar 27 boletines"
 */
export function t(key: MessageKey, params: MessageParams = {}): string {
  const entry = lookup(key)
  if (typeof entry === 'string') return interpolate(entry, params)
  const count = params['count']
  if (typeof count !== 'number') {
    throw new Error(`i18n key "${key}" is plural and needs a numeric "count" parameter`)
  }
  const category = pluralRules.select(count)
  return interpolate(category === 'one' ? entry.one : entry.other, params)
}
