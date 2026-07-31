import type { AxiosAdapter } from 'axios'
import { cleanup, render, screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, beforeEach, describe, expect, it } from 'vitest'

import { http } from '@/api/http'
import { Application } from '@/app/Application'
import { useAuthStore } from '@/stores/auth'
import { useMenuStore } from '@/stores/menu'

describe('Users workflow', () => {
  const originalAdapter = http.defaults.adapter

  beforeEach(() => {
    useAuthStore.getState().clearSession()
    useMenuStore.getState().resetAccess()
  })

  afterEach(() => {
    cleanup()
    http.defaults.adapter = originalAdapter
  })

  it('creates a user from the user list', async () => {
    const currentUser = {
      id: 1,
      userName: 'admin',
      nickName: 'Admin',
      homeRoute: 'users',
      roles: [{ id: 1, code: 'super_admin', name: 'Super Admin' }],
    }
    useAuthStore.getState().setSession({ accessToken: 'token', refreshToken: 'refresh', userInfo: currentUser })
    let createdPayload: Record<string, unknown> | null = null
    let userListCalls = 0
    http.defaults.adapter = (async (config) => {
      let data: unknown
      if (config.url === '/users/me') data = { code: 'OK', message: 'ok', data: { userInfo: currentUser } }
      else if (config.url === '/menus/current')
        data = {
          code: 'OK',
          message: 'ok',
          data: {
            menus: [{ name: 'users', path: 'users' }],
            permissions: ['system:user:create', 'system:role:list', 'system:user:assign-roles'],
          },
        }
      else if (config.url === '/roles')
        data = {
          code: 'OK',
          message: 'ok',
          data: {
            list: [
              {
                id: 2,
                code: 'operator',
                name: 'Operator',
                status: 'enabled',
                sort: 1,
              },
            ],
          },
        }
      else if (config.url === '/users' && config.method === 'post') {
        createdPayload = JSON.parse(String(config.data))
        data = { code: 'OK', message: 'created', data: { id: 9 } }
      } else if (config.url === '/users') {
        userListCalls += 1
        data = { code: 'OK', message: 'ok', data: { list: [], total: 0, page: 1, pageSize: 10 } }
      } else throw new Error(`Unexpected request: ${config.method} ${config.url}`)
      return { data, status: 200, statusText: 'OK', headers: {}, config }
    }) as AxiosAdapter
    window.history.replaceState({}, '', '/users')
    render(<Application />)

    const user = userEvent.setup()
    await user.click(await screen.findByRole('button', { name: 'New user' }))
    await user.type(screen.getByLabelText('Username'), 'new-operator')
    await user.type(screen.getByLabelText('Nickname'), 'New Operator')
    await user.clear(screen.getByLabelText('Password'))
    await user.type(screen.getByLabelText('Password'), 'safe-password')
    await user.click(screen.getByRole('checkbox', { name: 'Operator' }))
    await user.click(screen.getByRole('combobox', { name: 'Status' }))
    await user.click(await screen.findByRole('option', { name: 'Disabled' }))
    await user.click(screen.getByRole('button', { name: 'Create user' }))

    await screen.findByText('User created')
    expect(createdPayload).toMatchObject({
      username: 'new-operator',
      nickName: 'New Operator',
      password: 'safe-password',
      roleIds: [2],
      enable: 0,
    })
    expect(userListCalls).toBeGreaterThan(1)
  })

  it('searches users by keyword through the server list API', async () => {
    const currentUser = {
      id: 1,
      userName: 'admin',
      nickName: 'Admin',
      homeRoute: 'users',
      roles: [{ id: 1, code: 'super_admin', name: 'Super Admin' }],
    }
    useAuthStore.getState().setSession({ accessToken: 'token', refreshToken: 'refresh', userInfo: currentUser })
    const requestedKeywords: Array<string | undefined> = []
    http.defaults.adapter = (async (config) => {
      let data: unknown
      if (config.url === '/users/me') data = { code: 'OK', message: 'ok', data: { userInfo: currentUser } }
      else if (config.url === '/menus/current')
        data = { code: 'OK', message: 'ok', data: { menus: [{ name: 'users', path: 'users' }], permissions: [] } }
      else if (config.url === '/roles') data = { code: 'OK', message: 'ok', data: { list: [] } }
      else if (config.url === '/users') {
        const keyword = config.params?.keyword as string | undefined
        requestedKeywords.push(keyword)
        data = {
          code: 'OK',
          message: 'ok',
          data: {
            list: keyword
              ? [
                  {
                    id: 42,
                    userName: 'employee_42',
                    nickName: 'Matched User',
                    phone: '',
                    email: 'employee42@example.test',
                    enable: 1,
                    roles: [],
                  },
                ]
              : [],
            total: keyword ? 1 : 0,
            page: 1,
            pageSize: 10,
          },
        }
      } else throw new Error(`Unexpected request: ${config.method} ${config.url}`)
      return { data, status: 200, statusText: 'OK', headers: {}, config }
    }) as AxiosAdapter
    window.history.replaceState({}, '', '/users')
    render(<Application />)

    const user = userEvent.setup()
    await user.type(await screen.findByLabelText('Search users'), 'employee_42')
    await user.click(screen.getByRole('button', { name: 'Search' }))

    expect(await screen.findByText('Matched User')).toBeInTheDocument()
    expect(requestedKeywords).toContain('employee_42')
    expect(screen.queryByRole('button', { name: 'Access' })).not.toBeInTheDocument()
  })

  it('manages direct employee permissions separately from assigned roles', async () => {
    const currentUser = {
      id: 1,
      userName: 'admin',
      nickName: 'Admin',
      homeRoute: 'users',
      roles: [{ id: 2, code: 'operator', name: 'Operator' }],
    }
    const target = {
      id: 42,
      userName: 'developer',
      nickName: 'Developer',
      phone: '',
      email: '',
      enable: 1,
      roles: [
        { id: 2, code: 'developer', name: 'Developer' },
        { id: 3, code: 'legacy', name: 'Legacy Developer' },
      ],
      roleIds: [2, 3],
    }
    let savedDirect: string[] = []
    useAuthStore.getState().setSession({ accessToken: 'token', refreshToken: 'refresh', userInfo: currentUser })
    http.defaults.adapter = (async (config) => {
      let data: unknown
      if (config.url === '/users/me') data = { code: 'OK', message: 'ok', data: { userInfo: currentUser } }
      else if (config.url === '/menus/current')
        data = {
          code: 'OK',
          message: 'ok',
          data: {
            menus: [{ name: 'users', path: 'users' }],
            permissions: [
              'system:user:list',
              'system:role:list',
              'system:user:permissions-read',
              'system:user:permissions-update',
            ],
          },
        }
      else if (config.url === '/roles')
        data = {
          code: 'OK',
          message: 'ok',
          data: {
            list: [
              { id: 1, code: 'super_admin', name: 'Super Admin', status: 'enabled', sort: 0 },
              { id: 2, code: 'developer', name: 'Developer', status: 'enabled', sort: 1 },
              { id: 3, code: 'legacy', name: 'Legacy Developer', status: 'disabled', sort: 2 },
            ],
          },
        }
      else if (config.url === '/users')
        data = { code: 'OK', message: 'ok', data: { list: [target], total: 1, page: 1, pageSize: 10 } }
      else if (config.url === '/users/42/permissions' && config.method === 'get')
        data = {
          code: 'OK',
          message: 'ok',
          data: {
            roleIds: [2, 3],
            directPermissions: [],
            effectivePermissions: [
              {
                permission: 'system:user:list',
                direct: false,
                roles: [{ id: 2, code: 'developer', name: 'Developer' }],
              },
            ],
            catalog: [
              {
                permission: 'system:user:create',
                title: 'Create user',
                menuType: 'action',
                status: 'enabled',
                effectivelyEnabled: true,
                owningPageId: 11,
                owningPageTitle: 'Users',
                pageVisible: false,
              },
            ],
          },
        }
      else if (config.url === '/users/42/permissions' && config.method === 'put') {
        savedDirect = JSON.parse(String(config.data)).permissions
        data = { code: 'OK', message: 'saved' }
      } else throw new Error(`Unexpected request: ${config.method} ${config.url}`)
      return { data, status: 200, statusText: 'OK', headers: {}, config }
    }) as AxiosAdapter
    window.history.replaceState({}, '', '/users')
    render(<Application />)

    const user = userEvent.setup()
    await user.click(await screen.findByRole('button', { name: 'Access' }))
    expect(await screen.findByRole('tab', { name: 'Assigned Roles' })).toBeInTheDocument()
    expect(screen.getByText('Protected')).toBeInTheDocument()
    expect(screen.getByText('Dormant')).toBeInTheDocument()
    await user.click(screen.getByRole('tab', { name: 'Direct Permissions' }))
    const directPanel = await screen.findByRole('tabpanel')
    await user.click(within(directPanel).getByRole('checkbox'))
    expect(within(directPanel).getByText('Page not visible')).toBeInTheDocument()
    await user.click(within(directPanel).getByRole('button', { name: 'Save direct permissions' }))
    await waitFor(() => expect(savedDirect).toEqual(['system:user:create']))

    await user.click(screen.getByRole('tab', { name: 'Effective Permissions' }))
    expect(await screen.findByText('system:user:list')).toBeInTheDocument()
  })
})
