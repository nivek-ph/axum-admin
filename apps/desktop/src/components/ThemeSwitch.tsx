import { Moon, Sun } from 'lucide-react'
import { useTranslation } from 'react-i18next'

import { Button } from '@/components/ui/Button'
import { useThemeStore } from '@/stores/theme'

export function ThemeSwitch() {
  const { t } = useTranslation()
  const mode = useThemeStore((state) => state.mode)
  const toggleMode = useThemeStore((state) => state.toggleMode)

  return (
    <Button aria-label={mode === 'light' ? t('Dark mode') : t('Light mode')} onClick={toggleMode} variant="ghost">
      {mode === 'light' ? <Moon size={16} /> : <Sun size={16} />}
    </Button>
  )
}
