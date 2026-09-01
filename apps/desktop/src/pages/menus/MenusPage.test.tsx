import type { AxiosAdapter } from 'axios'
import { cleanup, render, screen } from '@testing-library/react'
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
    cleanup()
    http.defaults.adapter = originalAdapter
    await i18n.changeLanguage('en-US')
  })

  it('shows menu permissions as a read-only tree table', async () => {
    const currentUser = {
      id: 1,
      userName: 'admin',
      nickName: 'Admin',
      roles: [{ id: 1, code: 'super_admin', name: 'Super Admin' }],
    }
    useAuthStore.getState().setSession({ accessToken: 'token', refreshToken: 'refresh', userInfo: currentUser })
    http.defaults.adapter = (async (config) => {
      let data: unknown
      if (config.url === '/users/me') data = { code: 'OK', message: 'ok', data: { userInfo: currentUser } }
      else if (config.url === '/menus/current')
        data = { code: 'OK', message: 'ok', data: { menus: [{ name: 'menus', path: 'menus' }], permissions: [] } }
      else if (config.url === '/menus/tree')
        data = {
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
                  children: [
                    {
                      id: 3,
                      parentId: 2,
                      name: 'Create user',
                      sort: 1,
                      menuType: 'action',
                      permission: 'system:user:create',
                      meta: { title: 'Create user' },
                      children: [],
                    },
                  ],
                },
              ],
            },
          ],
        }
      else throw new Error(`Unexpected request: ${config.method} ${config.url}`)
      return { data, status: 200, statusText: 'OK', headers: {}, config }
    }) as AxiosAdapter
    window.history.replaceState({}, '', '/menus')
    render(<Application />)

    expect(await screen.findByRole('heading', { name: 'Access catalog' })).toBeInTheDocument()
    expect(
      screen.getByText('Definitions are managed by database migrations and are read-only here.'),
    ).toBeInTheDocument()
    expect(await screen.findByText('Organization')).toBeInTheDocument()
    expect(screen.getAllByText('Users').length).toBeGreaterThan(0)
    expect(await screen.findByText('system:user:list')).toBeInTheDocument()
    expect(screen.getByText('system:user:create')).toBeInTheDocument()
    expect(screen.getByText('Create user')).toBeInTheDocument()
    expect(screen.queryByText('API bindings')).not.toBeInTheDocument()
    expect(screen.queryByText('Basic operations')).not.toBeInTheDocument()
    expect(screen.queryByRole('button', { name: /new|edit|delete/i })).not.toBeInTheDocument()

    const user = userEvent.setup()
    await user.click(screen.getByRole('button', { name: 'Collapse Users' }))
    expect(screen.queryByText('system:user:create')).not.toBeInTheDocument()
    await user.click(screen.getByRole('button', { name: 'Expand Users' }))
    expect(screen.getByText('system:user:create')).toBeInTheDocument()

    await user.click(screen.getByRole('button', { name: 'Collapse all' }))
    expect(screen.queryByText('system:user:list')).not.toBeInTheDocument()
    await user.click(screen.getByRole('button', { name: 'Expand all' }))
    expect(screen.getByText('system:user:list')).toBeInTheDocument()
    expect(screen.getByText('system:user:create')).toBeInTheDocument()

    await user.click(screen.getByRole('button', { name: 'Collapse Organization' }))
    expect(screen.queryByText('system:user:list')).not.toBeInTheDocument()
    await user.click(screen.getByRole('button', { name: 'Expand Organization' }))
    expect(screen.getByText('system:user:list')).toBeInTheDocument()

    await user.click(screen.getByRole('button', { name: 'Switch language' }))
    expect(await screen.findByRole('heading', { name: '权限目录' })).toBeInTheDocument()
  })
})
