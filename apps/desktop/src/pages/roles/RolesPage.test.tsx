import type { AxiosAdapter } from 'axios'
import { cleanup, render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, beforeEach, describe, expect, it } from 'vitest'

import { http } from '@/api/http'
import { Application } from '@/app/Application'
import { useAuthStore } from '@/stores/auth'
import { useMenuStore } from '@/stores/menu'

const currentUser = { id: 1, userName: 'admin', nickName: 'Admin' }
const permissions = [
  'system:role:list',
  'system:role:create',
  'system:role:update',
  'system:role:delete',
  'system:role:access-read',
  'system:role:access-update',
]
const tree = [
  {
    id: 10,
    parentId: 0,
    path: '/organization',
    name: 'organization',
    sort: 1,
    menuType: 'directory',
    status: 'enabled',
    meta: { title: 'Organization' },
    children: [
      {
        id: 11,
        parentId: 10,
        path: '/users',
        name: 'users',
        sort: 1,
        menuType: 'page',
        status: 'enabled',
        permission: 'system:user:list',
        meta: { title: 'Users' },
        children: [
          {
            id: 1101,
            parentId: 11,
            path: '',
            name: 'users:create',
            sort: 1,
            menuType: 'action',
            status: 'enabled',
            permission: 'system:user:create',
            meta: { title: 'Create user' },
            children: [],
          },
        ],
      },
    ],
  },
]

function renderRoles(adapter: AxiosAdapter) {
  useAuthStore.getState().setSession({ accessToken: 'token', refreshToken: 'refresh', userInfo: currentUser })
  http.defaults.adapter = adapter
  window.history.replaceState({}, '', '/roles')
  return render(<Application />)
}

describe('Role Access workbench', () => {
  const originalAdapter = http.defaults.adapter
  beforeEach(() => {
    useAuthStore.getState().clearSession()
    useMenuStore.getState().resetAccess()
  })
  afterEach(() => {
    cleanup()
    http.defaults.adapter = originalAdapter
  })

  it('uses one tree and saves action selection with its owning page', async () => {
    let saved: string[] = []
    renderRoles(
      (async (config) => {
        let data: unknown
        if (config.url === '/users/me') data = { code: 'OK', message: 'ok', data: { userInfo: currentUser } }
        else if (config.url === '/menus/current') data = { code: 'OK', message: 'ok', data: { menus: [{ name: 'roles', path: '/roles' }], permissions } }
        else if (config.url === '/roles') data = { code: 'OK', message: 'ok', data: { list: [{ id: 2, code: 'developer', name: 'Developer', status: 'enabled', sort: 1 }] } }
        else if (config.url === '/roles/2/access' && config.method === 'put') {
          saved = JSON.parse(String(config.data)).permissions
          data = { code: 'OK', message: 'saved' }
        } else if (config.url === '/roles/2/access') data = { code: 'OK', message: 'ok', data: { permissions: saved, tree, protected: false } }
        else throw new Error(`Unexpected request: ${config.method} ${config.url}`)
        return { data, status: 200, statusText: 'OK', headers: {}, config }
      }) as AxiosAdapter,
    )

    expect(await screen.findAllByText('Developer')).toHaveLength(2)
    expect(screen.getByRole('tab', { name: 'Role Access' })).toBeInTheDocument()
    expect(screen.queryByRole('tab', { name: 'Page Access' })).not.toBeInTheDocument()
    expect(screen.queryByRole('tab', { name: 'Operation Permissions' })).not.toBeInTheDocument()
    await userEvent.click(await screen.findByRole('checkbox', { name: 'Create user access' }))
    expect(screen.getByRole('checkbox', { name: 'Users access' })).toBeChecked()
    await userEvent.click(screen.getByRole('button', { name: 'Save role access' }))
    await waitFor(() => expect(saved).toEqual(['system:user:create', 'system:user:list']))
  })

  it('renders protected Role Access and lifecycle controls read-only', async () => {
    renderRoles(
      (async (config) => {
        let data: unknown
        if (config.url === '/users/me') data = { code: 'OK', message: 'ok', data: { userInfo: currentUser } }
        else if (config.url === '/menus/current') data = { code: 'OK', message: 'ok', data: { menus: [{ name: 'roles', path: '/roles' }], permissions } }
        else if (config.url === '/roles') data = { code: 'OK', message: 'ok', data: { list: [{ id: 1, code: 'super_admin', name: 'Super Admin', status: 'enabled', sort: 0 }] } }
        else if (config.url === '/roles/1/access') data = { code: 'OK', message: 'ok', data: { permissions: ['system:user:list'], tree, protected: true } }
        else throw new Error(`Unexpected request: ${config.method} ${config.url}`)
        return { data, status: 200, statusText: 'OK', headers: {}, config }
      }) as AxiosAdapter,
    )

    expect(await screen.findByText('The protected super_admin access is read-only and maintained by migrations.')).toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'Edit' })).toBeDisabled()
    expect(screen.getByRole('button', { name: 'Delete' })).toBeDisabled()
    expect(screen.queryByRole('button', { name: 'Save role access' })).not.toBeInTheDocument()
  })
})
