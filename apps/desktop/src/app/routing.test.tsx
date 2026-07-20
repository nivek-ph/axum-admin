import type { AxiosAdapter } from 'axios'
import { render, screen } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it } from 'vitest'

import { Application } from './Application'
import { http } from '@/api/http'
import { useAuthStore } from '@/stores/auth'
import { useMenuStore } from '@/stores/menu'

describe('Admin Console routing', () => {
  const originalAdapter = http.defaults.adapter

  beforeEach(() => {
    useAuthStore.getState().clearSession()
    useMenuStore.getState().resetAccess()
  })

  afterEach(() => {
    http.defaults.adapter = originalAdapter
  })

  it('redirects an authenticated operator away from an unauthorized route', async () => {
    useAuthStore.getState().setSession({
      accessToken: 'access-token',
      refreshToken: 'refresh-token',
      userInfo: { id: 8, userName: 'operator', nickName: 'Operator' },
    })
    useMenuStore.getState().setAuthorizedMenus([{ name: 'dashboard', path: 'dashboard', meta: { title: 'Dashboard' } }])
    http.defaults.adapter = (async (config) => {
      const data =
        config.url === '/users/me'
          ? { code: 'OK', message: 'ok', data: { userInfo: { id: 8, userName: 'operator', nickName: 'Operator' } } }
          : { code: 'OK', message: 'ok', data: { menus: [{ name: 'dashboard', path: 'dashboard' }], permissions: [] } }
      return { data, status: 200, statusText: 'OK', headers: {}, config }
    }) as AxiosAdapter
    window.history.replaceState({}, '', '/roles')

    render(<Application />)

    expect(await screen.findByRole('heading', { name: 'Dashboard' })).toBeInTheDocument()
    expect(window.location.pathname).toBe('/dashboard')
  })

  it('uses the authorized user home route for the application root', async () => {
    const userInfo = { id: 8, userName: 'operator', nickName: 'Operator', homeRoute: 'files' }
    useAuthStore.getState().setSession({ accessToken: 'access-token', refreshToken: 'refresh-token', userInfo })
    useMenuStore.getState().setAuthorizedMenus([{ name: 'files', path: 'files' }])
    http.defaults.adapter = (async (config) => {
      const data =
        config.url === '/users/me'
          ? { code: 'OK', message: 'ok', data: { userInfo } }
          : config.url === '/menus/current'
            ? { code: 'OK', message: 'ok', data: { menus: [{ name: 'files', path: 'files' }], permissions: [] } }
            : { code: 'OK', message: 'ok', data: { list: [], total: 0, page: 1, pageSize: 10 } }
      return { data, status: 200, statusText: 'OK', headers: {}, config }
    }) as AxiosAdapter
    window.history.replaceState({}, '', '/')
    render(<Application />)
    expect(await screen.findByRole('heading', { name: 'Files' })).toBeInTheDocument()
    expect(window.location.pathname).toBe('/files')
  })
})
