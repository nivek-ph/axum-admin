import type { AxiosAdapter } from 'axios'
import { cleanup, render, screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, beforeEach, describe, expect, it } from 'vitest'

import { http } from '@/api/http'
import { Application } from '@/app/Application'
import i18n from '@/i18n'
import { useAuthStore } from '@/stores/auth'
import { useMenuStore } from '@/stores/menu'

describe('Roles workbench', () => {
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

  it('renders the complete role workbench in Chinese', async () => {
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
        data = {
          code: 'OK',
          message: 'ok',
          data: {
            menus: [{ name: 'roles', path: 'roles' }],
            permissions: [
              'system:role:update-permission',
              'system:role:permissions-read',
              'system:role:permissions-update',
            ],
          },
        }
      else if (config.url === '/menus/tree')
        data = {
          code: 'OK',
          message: 'ok',
          data: [
            {
              id: 10,
              parentId: 0,
              path: '/system',
              name: 'system',
              sort: 1,
              menuType: 'directory',
              meta: { title: 'System' },
              children: [
                {
                  id: 11,
                  parentId: 10,
                  path: '/users',
                  name: 'users',
                  sort: 1,
                  menuType: 'page',
                  meta: { title: 'Users' },
                  children: [
                    {
                      id: 1101,
                      parentId: 11,
                      path: '',
                      name: 'users:create',
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
      else if (config.url === '/roles/2/menus')
        data = {
          code: 'OK',
          message: 'ok',
          data: { menuIds: [], effectiveMenuIds: [], systemManaged: false },
        }
      else if (config.url === '/roles/2/permissions')
        data = {
          code: 'OK',
          message: 'ok',
          data: {
            permissions: [],
            systemManaged: false,
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
      else throw new Error(`Unexpected request: ${config.method} ${config.url}`)
      return { data, status: 200, statusText: 'OK', headers: {}, config }
    }) as AxiosAdapter
    await i18n.changeLanguage('zh-CN')
    window.history.replaceState({}, '', '/roles')
    render(<Application />)

    expect(
      await screen.findByRole('heading', { name: '管理页面访问、操作权限、数据范围和成员。', level: 1 }),
    ).toBeInTheDocument()
    await userEvent.click(screen.getByRole('tab', { name: '基础信息' }))
    const basicPanel = await screen.findByRole('tabpanel')
    expect(within(basicPanel).getByText('角色代码')).toBeInTheDocument()
    expect(within(basicPanel).getByText('状态')).toBeInTheDocument()
    expect(within(basicPanel).getByText('数据范围')).toBeInTheDocument()
    expect(within(basicPanel).getByText('排序')).toBeInTheDocument()

    await userEvent.click(screen.getByRole('tab', { name: '页面访问' }))
    const menuPanel = await screen.findByRole('tabpanel')
    expect(within(menuPanel).getByText('选择该角色可以访问的目录和页面。')).toBeInTheDocument()
    expect(within(menuPanel).getByText('系统管理')).toBeInTheDocument()
    expect(within(menuPanel).getByText('用户管理')).toBeInTheDocument()
    await userEvent.click(screen.getByRole('tab', { name: '操作权限' }))
    expect(await screen.findByText('创建用户')).toBeInTheDocument()
    expect(screen.getByText('页面不可见')).toBeInTheDocument()
  })

  it('saves page access and operation permissions independently', async () => {
    const currentUser = {
      id: 1,
      userName: 'admin',
      nickName: 'Admin',
      roles: [{ id: 1, code: 'super_admin', name: 'Super Admin' }],
    }
    useAuthStore.getState().setSession({ accessToken: 'token', refreshToken: 'refresh', userInfo: currentUser })
    let savedMenuIds: number[] = []
    let savedPermissions: string[] = []
    let permissionGets = 0
    const menuTree = [
      {
        id: 1,
        parentId: 0,
        path: 'system',
        name: 'system',
        sort: 1,
        meta: { title: 'System' },
        children: [
          {
            id: 2,
            parentId: 1,
            path: 'users',
            name: 'users',
            sort: 1,
            menuType: 'page',
            meta: { title: 'Users' },
            children: [
              {
                id: 20,
                parentId: 2,
                path: '',
                name: 'users:list',
                sort: 1,
                menuType: 'action',
                permission: 'system:user:list',
                meta: { title: 'List' },
                children: [],
              },
              {
                id: 21,
                parentId: 2,
                path: '',
                name: 'users:create',
                sort: 2,
                menuType: 'action',
                permission: 'system:user:create',
                meta: { title: 'Create' },
                children: [],
              },
            ],
          },
        ],
      },
    ]
    http.defaults.adapter = (async (config) => {
      let data: unknown
      if (config.url === '/users/me') data = { code: 'OK', message: 'ok', data: { userInfo: currentUser } }
      else if (config.url === '/menus/current')
        data = {
          code: 'OK',
          message: 'ok',
          data: {
            menus: [{ name: 'roles', path: 'roles' }],
            permissions: [
              'system:role:update-permission',
              'system:role:permissions-read',
              'system:role:permissions-update',
              'system:role:list-users',
              'system:user:list',
            ],
          },
        }
      else if (config.url === '/menus/tree') data = { code: 'OK', message: 'ok', data: menuTree }
      else if (config.url === '/roles/2/menus' && config.method === 'get')
        data = { code: 'OK', message: 'ok', data: { menuIds: [], effectiveMenuIds: [], systemManaged: false } }
      else if (config.url === '/roles/2/menus' && config.method === 'put') {
        savedMenuIds = JSON.parse(String(config.data)).menuIds
        data = { code: 'OK', message: 'saved' }
      } else if (config.url === '/roles/2/permissions' && config.method !== 'put') {
        permissionGets += 1
        data = {
          code: 'OK',
          message: 'ok',
          data: {
            permissions: [],
            systemManaged: false,
            catalog: [
              {
                permission: 'system:user:list',
                title: 'List',
                menuType: 'action',
                status: 'enabled',
                effectivelyEnabled: true,
                owningPageId: 2,
                owningPageTitle: 'Users',
                pageVisible: false,
              },
              {
                permission: 'system:user:create',
                title: 'Create',
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
      } else if (config.url === '/roles/2/permissions' && config.method === 'put') {
        savedPermissions = JSON.parse(String(config.data)).permissions
        data = { code: 'OK', message: 'saved' }
      } else if (config.url === '/roles')
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
      else throw new Error(`Unexpected request: ${config.method} ${config.url}`)
      return { data, status: 200, statusText: 'OK', headers: {}, config }
    }) as AxiosAdapter
    window.history.replaceState({}, '', '/roles')
    render(<Application />)

    const user = userEvent.setup()
    expect(await screen.findByRole('tab', { name: 'Basic Info' })).toBeInTheDocument()
    expect(screen.getByRole('tab', { name: 'Data Scope' })).toBeInTheDocument()
    expect(screen.getByRole('tab', { name: 'Assigned Users' })).toBeInTheDocument()
    await user.click(await screen.findByRole('checkbox', { name: 'Users page access' }))
    await user.click(screen.getByRole('button', { name: 'Save page access' }))
    await waitFor(() => expect(savedMenuIds).toEqual([1, 2]))

    await user.click(screen.getByRole('tab', { name: 'Operation Permissions' }))
    await waitFor(() => expect(permissionGets).toBe(1))
    const permissionPanel = await screen.findByRole('tabpanel')
    await user.click(await within(permissionPanel).findByRole('checkbox', { name: 'Create' }))
    await user.click(screen.getByRole('button', { name: 'Save permissions' }))
    await waitFor(() => expect(savedPermissions).toEqual(['system:user:create']))
    expect(savedMenuIds).toEqual([1, 2])
    expect(screen.getByRole('checkbox', { name: 'List' })).not.toBeChecked()
  })

  it('lets operators pick custom departments without first selecting the custom scope radio', async () => {
    const currentUser = {
      id: 1,
      userName: 'admin',
      nickName: 'Admin',
      roles: [{ id: 1, code: 'super_admin', name: 'Super Admin' }],
    }
    useAuthStore.getState().setSession({ accessToken: 'token', refreshToken: 'refresh', userInfo: currentUser })
    let savedScope: string | null = null
    let savedDeptIds: number[] = []
    http.defaults.adapter = (async (config) => {
      let data: unknown
      if (config.url === '/users/me') data = { code: 'OK', message: 'ok', data: { userInfo: currentUser } }
      else if (config.url === '/menus/current')
        data = { code: 'OK', message: 'ok', data: { menus: [{ name: 'roles', path: 'roles' }], permissions: [] } }
      else if (config.url === '/menus/tree') data = { code: 'OK', message: 'ok', data: [] }
      else if (config.url === '/depts' && config.method === 'get')
        data = {
          code: 'OK',
          message: 'ok',
          data: {
            list: [
              { id: 3, parent_id: null, name: 'Operations', code: 'ops', sort: 1, status: 'enabled', children: [] },
            ],
          },
        }
      else if (config.url === '/roles/2/depts' && config.method === 'get')
        data = { code: 'OK', message: 'ok', data: { deptIds: [] } }
      else if (config.url === '/roles/2/depts' && config.method === 'put') {
        savedDeptIds = JSON.parse(String(config.data)).deptIds
        data = { code: 'OK', message: 'saved' }
      } else if (config.url === '/roles/2' && config.method === 'put') {
        savedScope = JSON.parse(String(config.data)).data_scope
        data = { code: 'OK', message: 'saved' }
      } else if (config.url === '/roles/2/menus') data = { code: 'OK', message: 'ok', data: { menuIds: [] } }
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
      else throw new Error(`Unexpected request: ${config.method} ${config.url}`)
      return { data, status: 200, statusText: 'OK', headers: {}, config }
    }) as AxiosAdapter
    window.history.replaceState({}, '', '/roles')
    render(<Application />)

    const user = userEvent.setup()
    await screen.findByRole('tab', { name: 'Data Scope' })
    await user.click(screen.getByRole('tab', { name: 'Data Scope' }))
    await user.click(await screen.findByRole('checkbox', { name: 'Operations' }))
    expect(screen.getByRole('radio', { name: 'Custom departments' })).toBeChecked()
    await user.click(screen.getByRole('button', { name: 'Save data scope' }))
    await waitFor(() => {
      expect(savedScope).toBe('custom_depts')
      expect(savedDeptIds).toEqual([3])
    })
  })

  it('hides the Members section without both role-user and user-list visibility permissions', async () => {
    const currentUser = { id: 3, userName: 'operator', nickName: 'Operator', permissions: ['system:role:list-users'] }
    useAuthStore.getState().setSession({ accessToken: 'token', refreshToken: 'refresh', userInfo: currentUser })
    http.defaults.adapter = (async (config) => {
      let data: unknown
      if (config.url === '/users/me') data = { code: 'OK', message: 'ok', data: { userInfo: currentUser } }
      else if (config.url === '/menus/current')
        data = {
          code: 'OK',
          message: 'ok',
          data: { menus: [{ name: 'roles' }], permissions: ['system:role:list-users'] },
        }
      else if (config.url === '/menus/tree') data = { code: 'OK', message: 'ok', data: [] }
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
      else if (config.url === '/roles/2/menus') data = { code: 'OK', message: 'ok', data: { menuIds: [] } }
      else throw new Error(`Unexpected request: ${config.method} ${config.url}`)
      return { data, status: 200, statusText: 'OK', headers: {}, config }
    }) as AxiosAdapter
    window.history.replaceState({}, '', '/roles')
    render(<Application />)
    await screen.findByText('Operator')
    expect(screen.queryByRole('tab', { name: 'Assigned Users' })).not.toBeInTheDocument()
  })
})
