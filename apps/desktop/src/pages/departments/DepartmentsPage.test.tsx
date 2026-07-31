import type { AxiosAdapter } from 'axios'
import { act, cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react'
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
    cleanup()
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
        data = {
          code: 'OK',
          message: 'ok',
          data: {
            menus: [{ name: 'departments' }],
            permissions: ['system:dept:create', 'system:dept:update', 'system:dept:delete'],
          },
        }
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

    await screen.findByText('Operations')
    await user.click(screen.getByRole('button', { name: 'Add child' }))
    expect(await screen.findByLabelText('Parent department')).toHaveTextContent('Operations')
    await user.type(await screen.findByLabelText('Name'), 'Support')
    await user.type(screen.getByLabelText('Code'), 'support')
    await user.click(screen.getByRole('button', { name: 'Save' }))
    await waitFor(() => expect(created).toMatchObject({ parent_id: 3, name: 'Support', code: 'support' }))
    await waitFor(() => expect(reads).toBeGreaterThan(1))
  })

  it('lets operators pick a parent department from the tree instead of typing an ID', async () => {
    const user = userEvent.setup()
    const currentUser = {
      id: 1,
      userName: 'admin',
      nickName: 'Admin',
      roles: [{ id: 1, code: 'super_admin', name: 'Super Admin' }],
    }
    useAuthStore.getState().setSession({ accessToken: 'token', refreshToken: 'refresh', userInfo: currentUser })
    let created: Record<string, unknown> | null = null
    http.defaults.adapter = (async (config) => {
      let data: unknown
      if (config.url === '/users/me') data = { code: 'OK', message: 'ok', data: { userInfo: currentUser } }
      else if (config.url === '/menus/current')
        data = {
          code: 'OK',
          message: 'ok',
          data: {
            menus: [{ name: 'departments' }],
            permissions: ['system:dept:create', 'system:dept:update', 'system:dept:delete'],
          },
        }
      else if (config.url === '/depts' && config.method === 'get') {
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

    await screen.findByText('Operations')
    await user.click(screen.getByRole('button', { name: 'New department' }))
    expect(await screen.findByLabelText('Parent department')).toHaveTextContent('Root department')
    await user.click(screen.getByLabelText('Parent department'))
    await user.click(await screen.findByRole('option', { name: 'Operations' }))
    await user.type(screen.getByLabelText('Name'), 'Support')
    await user.type(screen.getByLabelText('Code'), 'support')
    await user.click(screen.getByRole('button', { name: 'Save' }))
    await waitFor(() => expect(created).toMatchObject({ parent_id: 3, name: 'Support', code: 'support' }))
  })

  it('switches to the organization chart and collapses a department subtree', async () => {
    const user = userEvent.setup()
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
            menus: [{ name: 'departments' }],
            permissions: ['system:dept:create', 'system:dept:update', 'system:dept:delete'],
          },
        }
      else if (config.url === '/depts' && config.method === 'get') {
        data = {
          code: 'OK',
          message: 'ok',
          data: {
            list: [
              {
                id: 1,
                parent_id: null,
                name: 'Head Office',
                code: 'head_office',
                sort: 0,
                status: 'enabled',
                children: [],
              },
              {
                id: 3,
                parent_id: null,
                name: 'Operations',
                code: 'ops',
                sort: 1,
                status: 'enabled',
                children: [
                  {
                    id: 4,
                    parent_id: 3,
                    name: 'Support',
                    code: 'support',
                    sort: 1,
                    status: 'enabled',
                    children: [
                      {
                        id: 5,
                        parent_id: 4,
                        name: 'Service Desk',
                        code: 'service_desk',
                        sort: 1,
                        status: 'enabled',
                        children: [],
                      },
                    ],
                  },
                ],
              },
            ],
          },
        }
      } else throw new Error(`Unexpected request: ${config.method} ${config.url}`)
      return { data, status: 200, statusText: 'OK', headers: {}, config }
    }) as AxiosAdapter
    window.history.replaceState({}, '', '/departments')
    render(<Application />)

    await screen.findByText('Operations')
    await user.click(screen.getByRole('button', { name: 'Collapse Operations' }))
    expect(screen.queryByText('Support')).not.toBeInTheDocument()
    expect(screen.queryByText('Service Desk')).not.toBeInTheDocument()
    await user.click(screen.getByRole('button', { name: 'Expand Operations' }))
    expect(screen.getByText('Support')).toBeVisible()
    expect(screen.getByText('Service Desk')).toBeVisible()

    await user.click(screen.getByRole('button', { name: 'More actions for Operations' }))
    expect(await screen.findByRole('menuitem', { name: 'Edit' })).toBeVisible()
    await user.click(await screen.findByRole('menuitem', { name: 'Delete' }))
    expect(
      await screen.findByText('Move or delete the 2 subordinate departments before deleting "Operations".'),
    ).toBeVisible()

    await user.click(screen.getByRole('tab', { name: 'Organization chart' }))

    expect(await screen.findByText('3 departments')).toBeVisible()
    expect(screen.queryByText('Head Office')).not.toBeInTheDocument()
    expect(screen.queryByText('1 child')).not.toBeInTheDocument()
    expect(screen.getByText('Service Desk')).toBeVisible()

    await user.hover(screen.getByRole('group', { name: 'Operations' }))
    expect(await screen.findByText('Department details')).toBeVisible()
    expect(screen.getByText('ops')).toBeVisible()
    fireEvent.click(screen.getByRole('button', { name: 'Add child to Operations' }))
    expect(await screen.findByLabelText('Parent department')).toHaveTextContent('Operations')
    await user.click(screen.getByRole('button', { name: 'Cancel' }))

    await user.click(screen.getByRole('button', { name: 'Fullscreen' }))
    expect(screen.getByRole('button', { name: 'Exit fullscreen' })).toBeVisible()
    await user.keyboard('{Escape}')
    expect(screen.getByRole('button', { name: 'Fullscreen' })).toBeVisible()

    await user.click(screen.getByRole('button', { name: 'Collapse Operations' }))

    expect(screen.queryByText('Support')).not.toBeInTheDocument()
    expect(screen.queryByText('Service Desk')).not.toBeInTheDocument()
  })

  it('shows only department actions granted to the current operator', async () => {
    const user = userEvent.setup()
    const currentUser = {
      id: 4,
      userName: 'org_operator',
      nickName: 'Organization Operator',
      roles: [{ id: 4, code: 'org_operator', name: 'Organization Operator' }],
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
            menus: [{ name: 'departments', menuType: 'page' }],
            permissions: ['system:dept:create', 'system:dept:update'],
          },
        }
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
      else throw new Error(`Unexpected request: ${config.method} ${config.url}`)
      return { data, status: 200, statusText: 'OK', headers: {}, config }
    }) as AxiosAdapter
    window.history.replaceState({}, '', '/departments')

    render(<Application />)

    await screen.findByText('Operations')
    expect(screen.getByRole('button', { name: 'New department' })).toBeVisible()
    expect(screen.getByRole('button', { name: 'Add child' })).toBeVisible()
    await user.click(screen.getByRole('button', { name: 'More actions for Operations' }))
    expect(await screen.findByRole('menuitem', { name: 'Edit' })).toBeVisible()
    expect(screen.queryByRole('menuitem', { name: 'Delete' })).not.toBeInTheDocument()
    await user.keyboard('{Escape}')

    act(() => {
      useAuthStore.getState().setUserAndPermissions(currentUser, ['system:dept:delete'])
    })

    await waitFor(() => expect(screen.queryByRole('button', { name: 'New department' })).not.toBeInTheDocument())
    expect(screen.queryByRole('button', { name: 'Add child' })).not.toBeInTheDocument()
    await user.click(screen.getByRole('button', { name: 'More actions for Operations' }))
    expect(await screen.findByRole('menuitem', { name: 'Delete' })).toBeVisible()
    expect(screen.queryByRole('menuitem', { name: 'Edit' })).not.toBeInTheDocument()
    await user.keyboard('{Escape}')

    await user.click(screen.getByRole('tab', { name: 'Organization chart' }))
    expect(screen.queryByRole('button', { name: 'Add child to Operations' })).not.toBeInTheDocument()
    expect(screen.queryByRole('button', { name: 'Edit Operations' })).not.toBeInTheDocument()
  })
})
