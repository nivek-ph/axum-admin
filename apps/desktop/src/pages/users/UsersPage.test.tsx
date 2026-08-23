import type { AxiosAdapter } from 'axios'
import { cleanup, render, screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, beforeEach, describe, expect, it } from 'vitest'

import { http } from '@/api/http'
import { Application } from '@/app/Application'
import { useAuthStore } from '@/stores/auth'
import { useMenuStore } from '@/stores/menu'

const currentUser = { id: 1, userName: 'admin', nickName: 'Admin', deptName: 'Head Office' }
const accessPermissions = [
  'system:user:list',
  'system:user:create',
  'system:user:access-read',
  'system:user:assign-roles',
  'system:role:list',
]

function adapter(onSave?: (ids: number[]) => void): AxiosAdapter {
  return (async (config) => {
    let data: unknown
    if (config.url === '/users/me') data = { code: 'OK', message: 'ok', data: { userInfo: currentUser } }
    else if (config.url === '/menus/current') data = { code: 'OK', message: 'ok', data: { menus: [{ name: 'users', path: '/users' }], permissions: accessPermissions } }
    else if (config.url === '/users') data = { code: 'OK', message: 'ok', data: { list: [{ id: 42, userName: 'employee', nickName: 'Employee', phone: '', email: '', enable: 1, roles: [{ id: 3, code: 'dormant', name: 'Dormant role' }] }], total: 1, page: 1, pageSize: 10 } }
    else if (config.url === '/roles') data = { code: 'OK', message: 'ok', data: { list: [{ id: 2, code: 'reader', name: 'Reader', status: 'enabled', sort: 1 }, { id: 3, code: 'dormant', name: 'Dormant role', status: 'disabled', sort: 2 }] } }
    else if (config.url === '/users/42/access') data = { code: 'OK', message: 'ok', data: { assignedRoles: [{ id: 3, code: 'dormant', name: 'Dormant role', status: 'disabled', sort: 2 }], effectivePermissions: [{ permission: 'system:user:list', roles: [{ id: 2, code: 'reader', name: 'Reader' }] }] } }
    else if (config.url === '/users/42/roles') {
      onSave?.(JSON.parse(String(config.data)).roleIds)
      data = { code: 'OK', message: 'saved' }
    } else throw new Error(`Unexpected request: ${config.method} ${config.url}`)
    return { data, status: 200, statusText: 'OK', headers: {}, config }
  }) as AxiosAdapter
}

function renderUsers(mock: AxiosAdapter) {
  useAuthStore.getState().setSession({ accessToken: 'token', refreshToken: 'refresh', userInfo: currentUser })
  http.defaults.adapter = mock
  window.history.replaceState({}, '', '/users')
  return render(<Application />)
}

describe('Role-only User Access', () => {
  const originalAdapter = http.defaults.adapter
  beforeEach(() => {
    useAuthStore.getState().clearSession()
    useMenuStore.getState().resetAccess()
  })
  afterEach(() => {
    cleanup()
    http.defaults.adapter = originalAdapter
  })

  it('shows Assigned Roles and read-only Effective Permissions without Direct Permissions', async () => {
    renderUsers(adapter())
    expect(await screen.findByRole('heading', { name: 'Manage employee accounts and role access.' })).toBeInTheDocument()
    await userEvent.click(await screen.findByRole('button', { name: 'Access' }))

    expect(screen.getByRole('tab', { name: 'Assigned Roles' })).toBeInTheDocument()
    expect(screen.getByRole('tab', { name: 'Effective Permissions' })).toBeInTheDocument()
    expect(screen.queryByRole('tab', { name: 'Direct Permissions' })).not.toBeInTheDocument()
    const rolesPanel = screen.getByRole('tabpanel', { name: 'Assigned Roles' })
    expect(within(rolesPanel).getByRole('checkbox', { name: /Dormant role/ })).toBeChecked()
    expect(within(rolesPanel).getByText('Dormant')).toBeInTheDocument()
    await userEvent.click(screen.getByRole('tab', { name: 'Effective Permissions' }))
    expect(screen.getByText('system:user:list')).toBeInTheDocument()
    expect(screen.getAllByText('Reader').length).toBeGreaterThan(0)
    expect(screen.queryByText('Direct')).not.toBeInTheDocument()
  })

  it('allows removing a dormant assignment and never enables adding it', async () => {
    let saved: number[] | undefined
    renderUsers(adapter((ids) => { saved = ids }))
    await userEvent.click(await screen.findByRole('button', { name: 'Access' }))
    const rolesPanel = screen.getByRole('tabpanel', { name: 'Assigned Roles' })
    const dormant = within(rolesPanel).getByRole('checkbox', { name: /Dormant role/ })
    expect(dormant).toBeEnabled()
    await userEvent.click(dormant)
    await userEvent.click(within(rolesPanel).getByRole('button', { name: 'Save roles' }))
    await waitFor(() => expect(saved).toEqual([]))
  })
})
