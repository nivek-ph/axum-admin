import type { AxiosAdapter } from 'axios'
import { render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, beforeEach, describe, expect, it } from 'vitest'

import { http } from '@/api/http'
import { Application } from '@/app/Application'
import { useAuthStore } from '@/stores/auth'
import { useMenuStore } from '@/stores/menu'

describe('Params workflow', () => {
  const originalAdapter = http.defaults.adapter
  beforeEach(() => {
    useAuthStore.getState().clearSession()
    useMenuStore.getState().resetAccess()
  })
  afterEach(() => {
    http.defaults.adapter = originalAdapter
  })

  it('edits and deletes a param with application confirmation and list refresh', async () => {
    const user = userEvent.setup()
    const currentUser = {
      id: 1,
      userName: 'admin',
      nickName: 'Admin',
      roles: [{ id: 1, code: 'super_admin', name: 'Super Admin' }],
    }
    useAuthStore.getState().setSession({ accessToken: 'token', refreshToken: 'refresh', userInfo: currentUser })
    let updated: Record<string, unknown> | null = null
    let deleted = false
    http.defaults.adapter = (async (config) => {
      let data: unknown
      if (config.url === '/users/me') data = { code: 'OK', message: 'ok', data: { userInfo: currentUser } }
      else if (config.url === '/menus/current')
        data = { code: 'OK', message: 'ok', data: { menus: [{ name: 'params' }], permissions: [] } }
      else if (config.url === '/params' && config.method === 'get')
        data = {
          code: 'OK',
          message: 'ok',
          data: {
            list: [{ id: 8, name: 'Theme', key: 'ui.theme', value: 'light', desc: '' }],
            total: 1,
            page: 1,
            pageSize: 10,
          },
        }
      else if (config.url === '/params/8' && config.method === 'put') {
        updated = JSON.parse(String(config.data))
        data = { code: 'OK', message: 'saved' }
      } else if (config.url === '/params/8' && config.method === 'delete') {
        deleted = true
        data = { code: 'OK', message: 'deleted' }
      } else throw new Error(`Unexpected request: ${config.method} ${config.url}`)
      return { data, status: 200, statusText: 'OK', headers: {}, config }
    }) as AxiosAdapter
    window.history.replaceState({}, '', '/params')
    render(<Application />)

    await user.click(await screen.findByRole('button', { name: 'Edit' }))
    await user.clear(screen.getByLabelText('Value'))
    await user.type(screen.getByLabelText('Value'), 'dark')
    await user.click(screen.getByRole('button', { name: 'Save' }))
    await waitFor(() => expect(updated).toMatchObject({ id: 8, key: 'ui.theme', value: 'dark' }))
    await user.click(await screen.findByRole('button', { name: 'Delete' }))
    await user.click(await screen.findByRole('button', { name: 'Confirm' }))
    await waitFor(() => expect(deleted).toBe(true))
  })
})
