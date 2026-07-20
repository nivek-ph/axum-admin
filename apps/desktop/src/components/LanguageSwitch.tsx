import { Languages } from 'lucide-react'
import { useTranslation } from 'react-i18next'

import { Button } from './ui/Button'
import { toggleLocale } from '@/i18n'

export function LanguageSwitch() {
  const { i18n } = useTranslation()
  return (
    <Button
      aria-label="Switch language"
      className="language-switch"
      onClick={() => void toggleLocale()}
      variant="ghost"
    >
      <Languages size={16} />
      {i18n.language === 'zh-CN' ? '中' : 'EN'}
    </Button>
  )
}
