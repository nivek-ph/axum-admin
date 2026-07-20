import type { AxiosAdapter } from 'axios'
import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, beforeEach, describe, expect, it } from 'vitest'

import { http } from '@/api/http'
import { Application } from '@/app/Application'
import { useAuthStore } from '@/stores/auth'
import { useMenuStore } from '@/stores/menu'
import i18n from '@/i18n'

describe('Access catalog', () => {
  const originalAdapter = http.defaults.adapter
  beforeEach(() => {
    useAuthStore.getState().clearSession()
    useMenuStore.getState().resetAccess()
  })
  afterEach(async () => {
    http.defaults.adapter = originalAdapter
    await i18n.changeLanguage('en-US')
  })

  it('shows menu permissions and API bindings as a read-only catalog', async () => {
    const currentUser = {
      id: 1,
      userName: 'admin',
      nickName: 'Admin',
      roles: [{ id: 1, code: 'super_admin', name: 'Super Admin' }],
    }
    useAuthStore.getState().setSession({ accessToken: 'token', refreshToken: 'refresh', userInfo: currentUser })
    http.defaults.adapter = (async (config) => {
      const data =
        config.url === '/users/me'
          ? { code: 'OK', message: 'ok', data: { userInfo: currentUser } }
          : config.url === '/menus/current'
            ? { code: 'OK', message: 'ok', data: { menus: [{ name: 'menus', path: 'menus' }], permissions: [] } }
            : {
                code: 'OK',
                message: 'ok',
                data: [
                  {
                    id: 1,
                    parentId: 0,
                    path: 'organization',
                    name: 'organization',
                    sort: 1,
                    menuType: 'directory',
                    meta: { title: 'Organization' },
                    children: [
                      {
                        id: 2,
                        parentId: 1,
                        path: 'users',
                        name: 'users',
                        sort: 1,
                        menuType: 'page',
                        permission: 'system:user:list',
                        meta: { title: 'Users' },
                        apiBindings: [{ method: 'GET', pathPattern: '/api/users' }],
                        children: [],
                      },
                    ],
                  },
                ],
              }
      return { data, status: 200, statusText: 'OK', headers: {}, config }
    }) as AxiosAdapter
    window.history.replaceState({}, '', '/menus')
    render(<Application />)

    expect(await screen.findByRole('heading', { name: 'Menus and permissions' })).toBeInTheDocument()
    expect(await screen.findByText('system:user:list')).toBeInTheDocument()
    expect(screen.getByText('/api/users')).toBeInTheDocument()
    expect(screen.queryByRole('button', { name: /new|edit|delete/i })).not.toBeInTheDocument()
    const user = userEvent.setup()
    await user.click(screen.getByRole('button', { name: 'Collapse Organization' }))
    expect(screen.queryByText('system:user:list')).not.toBeInTheDocument()
    await user.click(screen.getByRole('button', { name: 'Expand Organization' }))
    expect(screen.getByText('system:user:list')).toBeInTheDocument()
    await user.click(screen.getByRole('button', { name: 'Switch language' }))
    expect(await screen.findByRole('heading', { name: '菜单与权限' })).toBeInTheDocument()
  })
})
