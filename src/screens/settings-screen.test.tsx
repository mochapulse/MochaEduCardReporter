import { screen } from '@testing-library/react'
import { describe, expect, it } from 'vitest'
import { createMockApi, type MockApi } from '@/lib/api'
import { renderApp } from '@/test/render-app'

async function openSettings(api?: MockApi) {
  const app = renderApp(api ? { api } : {})
  await app.user.click(screen.getByRole('button', { name: 'Ajustes' }))
  await screen.findByRole('heading', { name: 'Ajustes' })
  return app
}

describe('Ajustes: actualizaciones', () => {
  it('says the app is current when the feed has nothing newer', async () => {
    const { user } = await openSettings()
    await user.click(screen.getByRole('button', { name: 'Buscar actualizaciones' }))
    expect(await screen.findByText('Tienes la última versión.')).toBeInTheDocument()
  })

  it('offers the announced version and installs it', async () => {
    const api = createMockApi({ stepDelayMs: 0, updateAvailable: '1.1.0' })
    const { user } = await openSettings(api)
    await user.click(screen.getByRole('button', { name: 'Buscar actualizaciones' }))
    expect(await screen.findByText('Hay una versión nueva: 1.1.0.')).toBeInTheDocument()
    await user.click(screen.getByRole('button', { name: 'Actualizar ahora' }))
    expect(api.calls.some((call) => call.method === 'installUpdate')).toBe(true)
  })

  it('shows what happened when the feed cannot be read', async () => {
    const api = createMockApi({ stepDelayMs: 0 })
    api.checkForUpdate = () => Promise.reject(new Error('network unreachable'))
    const { user } = await openSettings(api)
    await user.click(screen.getByRole('button', { name: 'Buscar actualizaciones' }))
    expect(await screen.findByText('Algo salió mal')).toBeInTheDocument()
    await user.click(screen.getByRole('button', { name: 'Ver detalles' }))
    expect(await screen.findByText(/network unreachable/)).toBeInTheDocument()
    // The button is usable again, so the person can retry.
    expect(screen.getByRole('button', { name: 'Buscar actualizaciones' })).toBeEnabled()
  })
})
