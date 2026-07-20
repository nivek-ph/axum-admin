import { beforeEach, describe, expect, it } from 'vitest'

import { buildMenuItems, useMenuStore } from './menu'
import { useAuthStore } from './auth'

describe('menu access state', () => {
  beforeEach(() => {
    useAuthStore.getState().clearSession()
    useMenuStore.getState().resetAccess()
  })

  it('maps nested server menus and falls back to the first authorized route', () => {
    const items = buildMenuItems([
      { name: 'system', children: [{ name: 'roles', path: 'roles' }] },
      { name: 'files', path: 'files' },
    ])
    expect(items.map((item) => item.key)).toEqual(['roles', 'files'])
    useMenuStore.getState().setAuthorizedMenus([{ name: 'files', path: 'files' }])
    expect(useMenuStore.getState().canAccess('roles')).toBe(false)
    expect(useMenuStore.getState().firstAuthorizedPath()).toBe('/files')
  })

  it('allows every core route for super admin and resets access state', () => {
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
    expect(useMenuStore.getState().canAccess('roles')).toBe(true)
    useMenuStore.getState().resetAccess()
    expect(useMenuStore.getState().accessLoaded).toBe(false)
  })
})
