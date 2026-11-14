import { render } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it } from 'vitest'
import { TooltipProvider } from '@/components/ui/tooltip'
import { createMemoryStore } from '@/lib/preferences'
import { TextSizeProvider } from '@/lib/text-size'
import { ThemeProvider } from '@/lib/theme'
import { expectNoA11yViolations } from '@/test/axe'
import { GalleryPage } from './gallery-page'

function mount() {
  const store = createMemoryStore()
  return render(
    <ThemeProvider store={store}>
      <TextSizeProvider store={store}>
        <TooltipProvider>
          <GalleryPage />
        </TooltipProvider>
      </TextSizeProvider>
    </ThemeProvider>,
  )
}

describe('Gallery', () => {
  const themes = ['Claro', 'Oscuro'] as const
  const sizes = ['Normal', 'Grande', 'Muy grande'] as const

  it.each(themes.flatMap((theme) => sizes.map((size) => [theme, size] as const)))(
    'has no axe violations: theme %s, text size %s',
    async (theme, size) => {
      const user = userEvent.setup()
      const { container, getAllByRole } = mount()
      await user.click(getAllByRole('button', { name: theme })[0] as HTMLElement)
      await user.click(getAllByRole('button', { name: size })[0] as HTMLElement)
      expect(document.documentElement.dataset['theme']).toBe(theme === 'Claro' ? 'light' : 'dark')
      await expectNoA11yViolations(container)
    },
    20_000,
  )
})
