import { createPinia, setActivePinia } from 'pinia'
import { describe, expect, it } from 'vitest'

import { buildCoreMenuItems, useMenuStore } from './menu'

describe('menu store helpers', () => {
  it('keeps core menu coverage before remote access rules are loaded', () => {
    const items = buildCoreMenuItems()

    expect(items.some((item) => item.key === 'dashboard')).toBe(true)
    expect(items.some((item) => item.key === 'roles')).toBe(true)
  })

  it('filters core menus to the remote menus available to the current role', () => {
    const items = buildCoreMenuItems([
      {
        name: 'users',
        path: 'system/users',
        meta: { title: 'Users' }
      },
      {
        name: 'dashboard',
        path: 'home',
        meta: { title: 'Workbench' }
      }
    ])

    expect(items).toContainEqual({
      key: 'dashboard',
      label: 'Dashboard',
      path: '/dashboard'
    })
    expect(items).toContainEqual({
      key: 'users',
      label: 'Users',
      path: '/users'
    })
    expect(items.some((item) => item.key === 'roles')).toBe(false)
  })

  it('uses frontend core labels instead of stale backend titles', () => {
    const items = buildCoreMenuItems([
      {
        name: 'roles',
        path: 'roles',
        meta: { title: 'Role' }
      }
    ])

    expect(items).toEqual([
      {
        key: 'roles',
        label: 'Roles',
        path: '/roles'
      }
    ])
  })

  it('supports nested remote menu records from the backend tree', () => {
    const items = buildCoreMenuItems([
      {
        name: 'system',
        meta: { title: 'System' },
        children: [
          {
            name: 'menus',
            path: 'menus',
            meta: { title: 'Menus' }
          }
        ]
      }
    ])

    expect(items).toEqual([
      {
        key: 'menus',
        label: 'Access catalog',
        path: '/menus'
      }
    ])
  })

  it('falls back to profile when a role has no authorized menus', () => {
    setActivePinia(createPinia())
    const menuStore = useMenuStore()

    menuStore.setAuthorizedMenus([])

    expect(menuStore.firstAuthorizedPath()).toBe('/profile')
    expect(menuStore.canAccessRouteName('dashboard')).toBe(false)
    expect(menuStore.canAccessRouteName('profile')).toBe(true)
  })
})
