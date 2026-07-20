import type { AxiosAdapter } from 'axios'
import { render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, beforeEach, describe, expect, it } from 'vitest'

import { http } from '@/api/http'
import { Application } from '@/app/Application'
import { useAuthStore } from '@/stores/auth'
import { useMenuStore } from '@/stores/menu'

describe('Departments workflow', () => {
  const originalAdapter = http.defaults.adapter
  beforeEach(() => {
    useAuthStore.getState().clearSession()
    useMenuStore.getState().resetAccess()
  })
  afterEach(() => {
    http.defaults.adapter = originalAdapter
  })

  it('creates a child department through the scoped API and reloads the tree', async () => {
    const user = userEvent.setup()
    const currentUser = {
      id: 1,
      userName: 'admin',
      nickName: 'Admin',
      roles: [{ id: 1, code: 'super_admin', name: 'Super Admin' }],
    }
    useAuthStore.getState().setSession({ accessToken: 'token', refreshToken: 'refresh', userInfo: currentUser })
    let created: Record<string, unknown> | null = null
    let reads = 0
    http.defaults.adapter = (async (config) => {
      let data: unknown
      if (config.url === '/users/me') data = { code: 'OK', message: 'ok', data: { userInfo: currentUser } }
      else if (config.url === '/menus/current')
        data = { code: 'OK', message: 'ok', data: { menus: [{ name: 'departments' }], permissions: [] } }
      else if (config.url === '/depts' && config.method === 'get') {
        reads += 1
        data = {
          code: 'OK',
          message: 'ok',
          data: {
            list: [
              { id: 3, parent_id: null, name: 'Operations', code: 'ops', sort: 1, status: 'enabled', children: [] },
            ],
          },
        }
      } else if (config.url === '/depts' && config.method === 'post') {
        created = JSON.parse(String(config.data))
        data = { code: 'OK', message: 'saved' }
      } else throw new Error(`Unexpected request: ${config.method} ${config.url}`)
      return { data, status: 200, statusText: 'OK', headers: {}, config }
    }) as AxiosAdapter
    window.history.replaceState({}, '', '/departments')
    render(<Application />)

    await user.click(await screen.findByRole('button', { name: 'Add child' }))
    await user.type(screen.getByLabelText('Name'), 'Support')
    await user.type(screen.getByLabelText('Code'), 'support')
    await user.click(screen.getByRole('button', { name: 'Save' }))
    await waitFor(() => expect(created).toMatchObject({ parent_id: 3, name: 'Support', code: 'support' }))
    await waitFor(() => expect(reads).toBeGreaterThan(1))
  })
})
