import { describe, expect, it } from 'vitest'
import { createScenario } from './dev-scenarios'

describe('development scenarios', () => {
  it('does nothing without ?demo, so a plain dev run uses the real backend', async () => {
    expect(await createScenario(new URLSearchParams())).toBeNull()
    expect(await createScenario(new URLSearchParams('theme=claro&size=grande'))).toBeNull()
  })

  it('opens the home screen with the mock API when ?demo is present', async () => {
    const scenario = await createScenario(new URLSearchParams('demo=home'))
    expect(scenario?.api.kind).toBe('mock')
    expect(scenario?.initial).toBeUndefined()
  })
})
