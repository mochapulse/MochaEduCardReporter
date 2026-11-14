import { t } from '@/i18n/es'

/** "Francés IV – Grupo 2 – 2025-2", skipping the parts the file did not have. */
export function describeRun(subject: string, group: string, period = ''): string {
  return [subject, group ? t('common.group', { group }) : '', period]
    .filter((part) => part.trim() !== '')
    .join(' – ')
}

/** Text of a metadata value, or the placeholder dash for an empty one. */
export function valueOrDash(value: string): string {
  return value.trim() === '' ? t('common.emptyValue') : value
}
