import { screen, waitFor } from '@testing-library/react'
import { expect } from 'vitest'
import type { RenderedApp } from './render-app'

/** Home: "Elegir archivo de notas" with a workbook that has two sheets, then the first sheet. */
export async function openFirstSheet({ user }: Pick<RenderedApp, 'user'>): Promise<void> {
  await user.click(screen.getByRole('button', { name: 'Elegir archivo de notas' }))
  await user.click(
    await screen.findByRole('button', { name: 'Usar esta hoja: FRANCAIS IV LE MATIN' }),
  )
  await screen.findByRole('heading', { name: 'Paso 1 — Revisar' })
}

/** Review: Continuar; then waits until step 2 has checked the folder and "Generar" is usable. */
export async function continueToSave({ user }: Pick<RenderedApp, 'user'>): Promise<void> {
  await user.click(screen.getByRole('button', { name: 'Continuar' }))
  await screen.findByRole('heading', { name: 'Paso 2 — Guardar' })
}

export async function waitForGenerateEnabled(name: RegExp | string): Promise<HTMLElement> {
  const button = await screen.findByRole('button', { name })
  await waitFor(() => {
    expect(button).toBeEnabled()
  })
  return button
}
