import { cleanup, render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, beforeEach, describe, expect, it } from 'vitest'
import { MemoryRouter, Route, Routes } from 'react-router-dom'

import i18n from '@/i18n'
import { useAuthStore } from '@/stores/auth'
import { useMenuStore } from '@/stores/menu'
import { applyTheme, useThemeStore } from '@/stores/theme'

import { AppLayout } from './AppLayout'

describe('AppLayout shell', () => {
  beforeEach(async () => {
    window.localStorage.removeItem('ava.sidebarCollapsed')
    window.localStorage.removeItem('ava.themeMode')
    useThemeStore.setState({ mode: 'light' })
    applyTheme('light')
    useAuthStore.getState().setSession({
      accessToken: 'token',
      refreshToken: 'refresh',
      userInfo: {
        id: 1,
        userName: 'admin',
        nickName: 'Admin',
        roles: [{ id: 1, code: 'super_admin', name: 'Super Admin' }],
      },
    })
    useMenuStore.getState().resetAccess()
    await i18n.changeLanguage('zh-CN')
  })

  afterEach(async () => {
    cleanup()
    window.localStorage.removeItem('ava.sidebarCollapsed')
    window.localStorage.removeItem('ava.themeMode')
    useAuthStore.getState().clearSession()
    await i18n.changeLanguage('en-US')
  })

  it('keeps collapse in the sidebar footer and toggles dark mode only', async () => {
    const user = userEvent.setup()
    const { container } = render(
      <MemoryRouter initialEntries={['/dashboard']}>
        <Routes>
          <Route element={<AppLayout />}>
            <Route path="/dashboard" element={<div>Dashboard content</div>} />
          </Route>
        </Routes>
      </MemoryRouter>,
    )

    expect(screen.getByText('核心管理')).toBeVisible()
    expect(screen.getByRole('link', { name: '用户管理' })).toBeVisible()
    expect(container.querySelector('.sidebar-footer')?.querySelector('[aria-label="收起"]')).toBeTruthy()
    expect(container.querySelector('.masthead-leading')?.querySelector('[aria-label="收起"]')).toBeFalsy()
    expect(screen.queryByRole('button', { name: '灰蓝' })).not.toBeInTheDocument()

    await user.click(screen.getByRole('button', { name: '收起' }))
    expect(container.querySelector('.app-shell')).toHaveClass('is-sidebar-collapsed')
    expect(screen.getByRole('button', { name: '展开' })).toBeVisible()

    await user.click(screen.getByRole('button', { name: '深色模式' }))
    expect(document.documentElement.dataset.mode).toBe('dark')
    expect(document.documentElement.dataset.palette).toBe('indigo')
    expect(window.localStorage.getItem('ava.themeMode')).toBe('dark')
  })
})
