import type { AxiosAdapter } from 'axios'
import { cleanup, render, screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, beforeEach, describe, expect, it } from 'vitest'

import { http } from '@/api/http'
import { Application } from '@/app/Application'
import { useAuthStore } from '@/stores/auth'
import { useMenuStore } from '@/stores/menu'

const currentUser = {
  id: 1,
  userName: 'admin',
  nickName: 'Admin',
  roles: [{ id: 1, code: 'super_admin', name: 'Super Admin' }],
}
const rolePermissions = [
  'system:role:menus-read',
  'system:role:update-permission',
  'system:role:permissions-read',
  'system:role:permissions-update',
  'system:menu:list',
]
const menuTree = [
  {
    id: 1,
    parentId: 0,
    path: '/system',
    name: 'system',
    sort: 1,
    menuType: 'directory',
    meta: { title: 'System' },
    children: [
      {
        id: 2,
        parentId: 1,
        path: '/users',
        name: 'users',
        sort: 1,
        menuType: 'page',
        meta: { title: 'Users' },
        children: [],
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

describe('Roles workbench', () => {
  const originalAdapter = http.defaults.adapter

  beforeEach(() => {
    useAuthStore.getState().clearSession()
    useMenuStore.getState().resetAccess()
  })

  afterEach(() => {
    cleanup()
    http.defaults.adapter = originalAdapter
  })

  it('keeps reusable role configuration focused on page and operation access', async () => {
    renderRoles(
      (async (config) => {
        let data: unknown
        if (config.url === '/users/me') data = { code: 'OK', message: 'ok', data: { userInfo: currentUser } }
        else if (config.url === '/menus/current')
          data = {
            code: 'OK',
            message: 'ok',
            data: { menus: [{ name: 'roles', path: '/roles' }], permissions: rolePermissions },
          }
        else if (config.url === '/roles')
          data = {
            code: 'OK',
            message: 'ok',
            data: { list: [{ id: 2, code: 'developer', name: 'Developer', status: 'enabled', sort: 1 }] },
          }
        else if (config.url === '/menus/tree') data = { code: 'OK', message: 'ok', data: menuTree }
        else if (config.url === '/roles/2/menus')
          data = { code: 'OK', message: 'ok', data: { menuIds: [1, 2], effectiveMenuIds: [1, 2], protected: false } }
        else if (config.url === '/roles/2/permissions')
          data = {
            code: 'OK',
            message: 'ok',
            data: {
              permissions: [],
              protected: false,
              catalog: [
                {
                  permission: 'system:user:list',
                  title: 'List users',
                  menuType: 'page',
                  status: 'enabled',
                  effectivelyEnabled: true,
                  owningPageId: 2,
                  owningPageTitle: 'Users',
                  pageVisible: true,
                },
                {
                  permission: 'system:user:create',
                  title: 'Create user',
                  menuType: 'action',
                  status: 'enabled',
                  effectivelyEnabled: true,
                  owningPageId: 2,
                  owningPageTitle: 'Users',
                  pageVisible: true,
                },
              ],
            },
          }
        else throw new Error(`Unexpected request: ${config.method} ${config.url}`)
        return { data, status: 200, statusText: 'OK', headers: {}, config }
      }) as AxiosAdapter,
    )

    expect(await screen.findAllByText('Developer')).toHaveLength(2)
    expect(screen.getByRole('tab', { name: 'Page Access' })).toBeInTheDocument()
    expect(screen.getByRole('tab', { name: 'Operation Permissions' })).toBeInTheDocument()
    expect(screen.queryByRole('tab', { name: 'Data Scope' })).not.toBeInTheDocument()
    expect(screen.queryByRole('tab', { name: 'Assigned Users' })).not.toBeInTheDocument()

    await userEvent.click(screen.getByRole('tab', { name: 'Operation Permissions' }))
    expect(await screen.findByText('Create user')).toBeInTheDocument()
    expect(screen.queryByText('List users')).not.toBeInTheDocument()
  })

  it('saves page access and operation permissions independently', async () => {
    let savedMenuIds: number[] = []
    let savedPermissions: string[] = []
    renderRoles(
      (async (config) => {
        let data: unknown
        if (config.url === '/users/me') data = { code: 'OK', message: 'ok', data: { userInfo: currentUser } }
        else if (config.url === '/menus/current')
          data = {
            code: 'OK',
            message: 'ok',
            data: { menus: [{ name: 'roles', path: '/roles' }], permissions: rolePermissions },
          }
        else if (config.url === '/roles')
          data = {
            code: 'OK',
            message: 'ok',
            data: { list: [{ id: 2, code: 'developer', name: 'Developer', status: 'enabled', sort: 1 }] },
          }
        else if (config.url === '/menus/tree') data = { code: 'OK', message: 'ok', data: menuTree }
        else if (config.url === '/roles/2/menus' && config.method === 'get')
          data = { code: 'OK', message: 'ok', data: { menuIds: [], effectiveMenuIds: [], protected: false } }
        else if (config.url === '/roles/2/menus' && config.method === 'put') {
          savedMenuIds = JSON.parse(String(config.data)).menuIds
          data = { code: 'OK', message: 'saved' }
        } else if (config.url === '/roles/2/permissions' && config.method === 'get')
          data = {
            code: 'OK',
            message: 'ok',
            data: {
              permissions: [],
              protected: false,
              catalog: [
                {
                  permission: 'system:user:create',
                  title: 'Create user',
                  menuType: 'action',
                  status: 'enabled',
                  effectivelyEnabled: true,
                  owningPageId: 2,
                  owningPageTitle: 'Users',
                  pageVisible: false,
                },
              ],
            },
          }
        else if (config.url === '/roles/2/permissions' && config.method === 'put') {
          savedPermissions = JSON.parse(String(config.data)).permissions
          data = { code: 'OK', message: 'saved' }
        } else throw new Error(`Unexpected request: ${config.method} ${config.url}`)
        return { data, status: 200, statusText: 'OK', headers: {}, config }
      }) as AxiosAdapter,
    )

    const user = userEvent.setup()
    await user.click(await screen.findByRole('checkbox', { name: 'Users page access' }))
    await user.click(screen.getByRole('button', { name: 'Save page access' }))
    await waitFor(() => expect(savedMenuIds).toEqual([1, 2]))

    await user.click(screen.getByRole('tab', { name: 'Operation Permissions' }))
    const panel = await screen.findByRole('tabpanel')
    await user.click(await within(panel).findByRole('checkbox'))
    await user.click(screen.getByRole('button', { name: 'Save permissions' }))
    await waitFor(() => expect(savedPermissions).toEqual(['system:user:create']))
  })

  it('clears stale permissions while switching roles', async () => {
    let resolveAuditor: (() => void) | undefined
    renderRoles(
      (async (config) => {
        let data: unknown
        if (config.url === '/users/me') data = { code: 'OK', message: 'ok', data: { userInfo: currentUser } }
        else if (config.url === '/menus/current')
          data = {
            code: 'OK',
            message: 'ok',
            data: { menus: [{ name: 'roles', path: '/roles' }], permissions: rolePermissions },
          }
        else if (config.url === '/roles')
          data = {
            code: 'OK',
            message: 'ok',
            data: {
              list: [
                { id: 2, code: 'developer', name: 'Developer', status: 'enabled', sort: 1 },
                { id: 3, code: 'auditor', name: 'Auditor', status: 'enabled', sort: 2 },
              ],
            },
          }
        else if (config.url === '/menus/tree') data = { code: 'OK', message: 'ok', data: menuTree }
        else if (config.url === '/roles/2/menus')
          data = { code: 'OK', message: 'ok', data: { menuIds: [], effectiveMenuIds: [], protected: false } }
        else if (config.url === '/roles/2/permissions')
          data = {
            code: 'OK',
            message: 'ok',
            data: {
              permissions: ['system:user:create'],
              protected: false,
              catalog: [
                {
                  permission: 'system:user:create',
                  title: 'Create user',
                  menuType: 'action',
                  status: 'enabled',
                  effectivelyEnabled: true,
                  owningPageId: 2,
                  owningPageTitle: 'Users',
                  pageVisible: true,
                },
              ],
            },
          }
        else if (config.url === '/roles/3/permissions') {
          await new Promise<void>((resolve) => {
            resolveAuditor = resolve
          })
          data = {
            code: 'OK',
            message: 'ok',
            data: {
              permissions: [],
              protected: false,
              catalog: [
                {
                  permission: 'system:audit:list',
                  title: 'List audit logs',
                  menuType: 'action',
                  status: 'enabled',
                  effectivelyEnabled: true,
                  owningPageId: 3,
                  owningPageTitle: 'Audit',
                  pageVisible: true,
                },
              ],
            },
          }
        } else throw new Error(`Unexpected request: ${config.method} ${config.url}`)
        return { data, status: 200, statusText: 'OK', headers: {}, config }
      }) as AxiosAdapter,
    )

    const user = userEvent.setup()
    await user.click(await screen.findByRole('tab', { name: 'Operation Permissions' }))
    expect(await screen.findByText('Create user')).toBeInTheDocument()

    await user.click(screen.getByRole('button', { name: /Auditor/ }))
    await waitFor(() => expect(resolveAuditor).toBeTypeOf('function'))
    expect(screen.queryByText('Create user')).not.toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'Save permissions' })).toBeDisabled()

    resolveAuditor?.()
    expect(await screen.findByText('List audit logs')).toBeInTheDocument()
    expect(screen.queryByText('Create user')).not.toBeInTheDocument()
  })

  it('renders protected super_admin grants as read-only concrete assignments', async () => {
    renderRoles(
      (async (config) => {
        let data: unknown
        if (config.url === '/users/me') data = { code: 'OK', message: 'ok', data: { userInfo: currentUser } }
        else if (config.url === '/menus/current')
          data = {
            code: 'OK',
            message: 'ok',
            data: { menus: [{ name: 'roles', path: '/roles' }], permissions: rolePermissions },
          }
        else if (config.url === '/roles')
          data = {
            code: 'OK',
            message: 'ok',
            data: { list: [{ id: 1, code: 'super_admin', name: 'Super Admin', status: 'enabled', sort: 0 }] },
          }
        else if (config.url === '/menus/tree') data = { code: 'OK', message: 'ok', data: menuTree }
        else if (config.url === '/roles/1/menus')
          data = { code: 'OK', message: 'ok', data: { menuIds: [1, 2], effectiveMenuIds: [1, 2], protected: true } }
        else throw new Error(`Unexpected request: ${config.method} ${config.url}`)
        return { data, status: 200, statusText: 'OK', headers: {}, config }
      }) as AxiosAdapter,
    )

    expect(await screen.findAllByText('Super Admin')).toHaveLength(2)
    expect(await screen.findByText('The protected super_admin grants are maintained by migrations.')).toBeInTheDocument()
    expect(screen.getByRole('checkbox', { name: 'Users page access' })).toHaveAttribute('aria-disabled', 'true')
    expect(screen.queryByRole('button', { name: 'Save page access' })).not.toBeInTheDocument()
  })
})
