import { Activity, FolderKanban, ShieldCheck, Users } from 'lucide-react'
import { useTranslation } from 'react-i18next'

import { useAuthStore } from '@/stores/auth'

export function DashboardPage() {
  const { t } = useTranslation()
  const user = useAuthStore((state) => state.userInfo)
  return (
    <div className="page-stack">
      <header className="page-header">
        <div>
          <p className="eyebrow">{t('Operator overview')}</p>
          <h1>{t('Dashboard')}</h1>
          <p>
            {t('Welcome back')}, {user?.nickName || user?.userName}.
          </p>
        </div>
      </header>
      <div className="metric-grid">
        <article className="metric-card">
          <Users />
          <span>{t('Identity')}</span>
          <strong>{t('Users & roles')}</strong>
        </article>
        <article className="metric-card">
          <ShieldCheck />
          <span>{t('Access')}</span>
          <strong>{t('Policy aware')}</strong>
        </article>
        <article className="metric-card">
          <FolderKanban />
          <span>{t('Resources')}</span>
          <strong>{t('Configuration')}</strong>
        </article>
        <article className="metric-card">
          <Activity />
          <span>{t('Traceability')}</span>
          <strong>{t('Audit ready')}</strong>
        </article>
      </div>
    </div>
  )
}
