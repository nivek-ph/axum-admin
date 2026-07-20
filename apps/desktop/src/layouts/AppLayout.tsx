import {
  Activity,
  BookMarked,
  Building2,
  FileStack,
  LayoutDashboard,
  LogOut,
  PanelLeft,
  PanelLeftClose,
  Settings2,
  Shield,
  ShieldCheck,
  UserRound,
  Users,
  type LucideIcon,
} from 'lucide-react'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { NavLink, Outlet, useNavigate } from 'react-router-dom'
import { toast } from 'sonner'

import { logout } from '@/api/auth'
import { BrandMark } from '@/components/BrandMark'
import { LanguageSwitch } from '@/components/LanguageSwitch'
import { ThemeSwitch } from '@/components/ThemeSwitch'
import { Button } from '@/components/ui/Button'
import { useAuthStore } from '@/stores/auth'
import { useMenuStore } from '@/stores/menu'

const SIDEBAR_COLLAPSED_KEY = 'ava.sidebarCollapsed'

const MENU_ICONS: Record<string, LucideIcon> = {
  dashboard: LayoutDashboard,
  users: Users,
  roles: Shield,
  departments: Building2,
  menus: ShieldCheck,
  params: Settings2,
  dictionaries: BookMarked,
  files: FileStack,
  'audit-events': Activity,
  profile: UserRound,
}

export function AppLayout() {
  const { t } = useTranslation()
  const navigate = useNavigate()
  const items = useMenuStore((state) => state.items)
  const user = useAuthStore((state) => state.userInfo)
  const [collapsed, setCollapsed] = useState(() => window.localStorage.getItem(SIDEBAR_COLLAPSED_KEY) === '1')

  function toggleSidebar() {
    setCollapsed((current) => {
      const next = !current
      window.localStorage.setItem(SIDEBAR_COLLAPSED_KEY, next ? '1' : '0')
      return next
    })
  }

  async function signOut() {
    try {
      await logout()
    } catch {
      toast.warning(t('Server session may still be active'))
    }
    useAuthStore.getState().clearSession()
    useMenuStore.getState().resetAccess()
    navigate('/login', { replace: true })
  }

  return (
    <div className={`app-shell${collapsed ? ' is-sidebar-collapsed' : ''}`}>
      <header className="masthead">
        <div className="masthead-leading">
          <div className="masthead-brand">
            <BrandMark size="small" />
            <strong className="masthead-title">{t('Admin Console')}</strong>
          </div>
        </div>
        <div className="masthead-actions">
          <ThemeSwitch />
          <LanguageSwitch />
          <NavLink className="user-chip" to="/profile">
            <span>{user?.nickName || user?.userName}</span>
          </NavLink>
          <Button onClick={() => void signOut()} variant="ghost">
            <LogOut size={16} />
            {t('Sign out')}
          </Button>
        </div>
      </header>
      <div className="app-body">
        <aside className={`sidebar${collapsed ? ' is-collapsed' : ''}`}>
          <div className="sidebar-section">
            <div className="sidebar-caption">{t('Core')}</div>
            <nav aria-label="Main navigation">
              {items.map((item) => {
                const Icon = MENU_ICONS[item.key] ?? LayoutDashboard
                return (
                  <NavLink aria-label={t(item.label)} data-label={t(item.label)} key={item.key} to={item.path}>
                    <Icon className="nav-icon" size={15} />
                    <span className="nav-label">{t(item.label)}</span>
                  </NavLink>
                )
              })}
            </nav>
          </div>
          <div className="sidebar-footer">
            <Button
              aria-expanded={!collapsed}
              aria-label={collapsed ? t('Expand') : t('Collapse')}
              className="sidebar-collapse"
              data-label={collapsed ? t('Expand') : t('Collapse')}
              onClick={toggleSidebar}
              variant="ghost"
            >
              {collapsed ? <PanelLeft size={16} /> : <PanelLeftClose size={16} />}
              <span className="nav-label">{collapsed ? t('Expand') : t('Collapse')}</span>
            </Button>
          </div>
        </aside>
        <main className="content">
          <Outlet />
        </main>
      </div>
    </div>
  )
}
