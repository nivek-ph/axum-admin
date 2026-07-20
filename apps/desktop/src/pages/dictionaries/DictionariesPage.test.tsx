import type { AxiosAdapter } from 'axios'
import { render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, beforeEach, describe, expect, it } from 'vitest'

import { http } from '@/api/http'
import { Application } from '@/app/Application'
import { useAuthStore } from '@/stores/auth'
import { useMenuStore } from '@/stores/menu'

describe('Dictionaries workflow', () => {
  const originalAdapter = http.defaults.adapter
  beforeEach(() => {
    useAuthStore.getState().clearSession()
    useMenuStore.getState().resetAccess()
  })
  afterEach(() => {
    http.defaults.adapter = originalAdapter
  })

  it('creates a root detail within the selected dictionary and reloads its tree', async () => {
    const user = userEvent.setup()
    const currentUser = {
      id: 1,
      userName: 'admin',
      nickName: 'Admin',
      roles: [{ id: 1, code: 'super_admin', name: 'Super Admin' }],
    }
    useAuthStore.getState().setSession({ accessToken: 'token', refreshToken: 'refresh', userInfo: currentUser })
    let treeReads = 0
    let posted: { url?: string; body?: Record<string, unknown> } = {}
    http.defaults.adapter = (async (config) => {
      let data: unknown
      if (config.url === '/users/me') data = { code: 'OK', message: 'ok', data: { userInfo: currentUser } }
      else if (config.url === '/menus/current')
        data = {
          code: 'OK',
          message: 'ok',
          data: { menus: [{ name: 'dictionaries', path: 'dictionaries' }], permissions: [] },
        }
      else if (config.url === '/dictionaries' && config.method === 'get')
        data = {
          code: 'OK',
          message: 'ok',
          data: [{ id: 7, name: 'Status', type: 'system_status', status: true, desc: '', parentId: null }],
        }
      else if (config.url === '/dictionaries/7/tree' && config.method === 'get') {
        treeReads += 1
        data = { code: 'OK', message: 'ok', data: { list: [] } }
      } else if (config.url === '/dictionaries/7/tree' && config.method === 'post') {
        posted = { url: config.url, body: JSON.parse(String(config.data)) as Record<string, unknown> }
        data = { code: 'OK', message: 'ok', data: {} }
      } else data = { code: 'OK', message: 'ok', data: [] }
      return { data, status: 200, statusText: 'OK', headers: {}, config }
    }) as AxiosAdapter
    window.history.replaceState({}, '', '/dictionaries')
    render(<Application />)

    expect(await screen.findByRole('heading', { name: 'Status' })).toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'Search' })).toBeInTheDocument()
    await user.click(screen.getByRole('button', { name: /new root detail/i }))
    await user.type(screen.getByLabelText('Label'), 'Enabled')
    await user.type(screen.getByLabelText('Value'), 'enabled')
    await user.click(screen.getByRole('button', { name: 'Save' }))

    await waitFor(() => expect(posted.url).toBe('/dictionaries/7/tree'))
    expect(posted.body).toMatchObject({ label: 'Enabled', value: 'enabled', sysDictionaryId: 7, parentId: null })
    await waitFor(() => expect(treeReads).toBeGreaterThan(1))
  })
})
