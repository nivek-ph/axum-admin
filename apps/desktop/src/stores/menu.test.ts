import { beforeEach, describe, expect, it } from 'vitest'

import { buildMenuItems, useMenuStore } from './menu'
import { useAuthStore } from './auth'

describe('menu access state', () => {
  beforeEach(() => {
    useAuthStore.getState().clearSession()
    useMenuStore.getState().resetAccess()
  })

  it('preserves the authorized catalog hierarchy and falls back to the first page', () => {
    const items = buildMenuItems([
      {
        name: 'organization',
        meta: { title: 'Organization' },
        children: [{ name: 'roles', path: '/roles', meta: { title: 'Roles' } }],
      },
      { name: 'files', path: 'files' },
    ])
    expect(items).toEqual([
      {
        key: 'organization',
        label: 'Organization',
        children: [{ key: 'roles', label: 'Roles', path: '/roles', children: [] }],
      },
      { key: 'files', label: 'Files', path: '/files', children: [] },
    ])
    useMenuStore.getState().setAuthorizedMenus([{ name: 'files', path: 'files' }])
    expect(useMenuStore.getState().canAccess('roles')).toBe(false)
    expect(useMenuStore.getState().firstAuthorizedPath()).toBe('/files')
  })

  it('does not bypass concrete navigation grants for super admin and resets access state', () => {
    useAuthStore.getState().setSession({
      accessToken: 'a',
      refreshToken: 'r',
      userInfo: {
        id: 1,
        userName: 'admin',
        nickName: 'Admin',
        roles: [{ id: 1, code: 'super_admin', name: 'Super Admin' }],
      },
    })
    useMenuStore.getState().setAuthorizedMenus([])
    expect(useMenuStore.getState().canAccess('roles')).toBe(false)
    useMenuStore.getState().resetAccess()
    expect(useMenuStore.getState().accessLoaded).toBe(false)
  })
})
