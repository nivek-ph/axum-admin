import type { AxiosAdapter } from 'axios'
import { render, screen } from '@testing-library/react'
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
        data = { code: 'OK', message: 'ok', data: { menus: [{ name: 'users', path: 'users' }], permissions: [] } }
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
                data_scope: 'all',
                is_system: false,
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
    await user.selectOptions(screen.getByLabelText('Role'), '2')
    await user.selectOptions(screen.getByLabelText('Status'), '0')
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
})
