import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it, vi } from 'vitest'

import { getMenu, getUserInfo } from '@/api/auth'
import { writeAuthSession } from './authStorage'
import { useAuthStore } from './auth'
import { bootstrapAuthSession } from './bootstrapAuth'
import { useMenuStore } from './menu'

vi.mock('@/api/auth', () => ({
  getUserInfo: vi.fn(),
  getMenu: vi.fn()
}))

describe('bootstrapAuthSession', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('restores the current user menu permissions from the backend', async () => {
    setActivePinia(createPinia())
    vi.mocked(getUserInfo).mockResolvedValue({
      code: 'OK',
      message: 'ok',
      data: {
        userInfo: {
          id: 1,
          userName: 'operator',
          nickName: 'Operator',
          roles: [{ id: 999, code: 'operator', name: 'Operator' }]
        }
      }
    })
    vi.mocked(getMenu).mockResolvedValue({
      code: 'OK',
      message: 'ok',
      data: {
        menus: [{ name: 'dashboard', path: 'dashboard', meta: { title: 'Dashboard' } }]
      }
    })
    writeAuthSession({
      token: 'token-123',
      userInfo: {
        id: 1,
        userName: 'operator',
        nickName: 'Operator'
      }
    })

    await bootstrapAuthSession()

    const menuStore = useMenuStore()
    expect(menuStore.items).toEqual([{ key: 'dashboard', label: 'Dashboard', path: '/dashboard' }])
    expect(menuStore.canAccessRouteName('roles')).toBe(false)
  })

  it('treats super_admin role code as full menu access during bootstrap', async () => {
    setActivePinia(createPinia())
    vi.mocked(getUserInfo).mockResolvedValue({
      code: 'OK',
      message: 'ok',
      data: {
        userInfo: {
          id: 1,
          userName: 'admin',
          nickName: 'Admin',
          homeRoute: 'dashboard',
          roles: [{ id: 1, code: 'super_admin', name: 'Super Admin' }]
        }
      }
    })
    vi.mocked(getMenu).mockResolvedValue({
      code: 'OK',
      message: 'ok',
      data: {
        menus: [{ name: 'dashboard', path: 'dashboard', meta: { title: 'Dashboard' } }]
      }
    })
    writeAuthSession({
      token: 'token-123',
      userInfo: {
        id: 1,
        userName: 'admin',
        nickName: 'Admin'
      }
    })

    await bootstrapAuthSession()

    const menuStore = useMenuStore()
    expect(menuStore.canAccessRouteName('roles')).toBe(true)
  })

  it('clears the session when current user details are missing', async () => {
    setActivePinia(createPinia())
    vi.mocked(getUserInfo).mockResolvedValue({
      code: 'OK',
      message: 'ok',
      data: {}
    } as Awaited<ReturnType<typeof getUserInfo>>)
    vi.mocked(getMenu).mockResolvedValue({
      code: 'OK',
      message: 'ok',
      data: {
        menus: [{ name: 'dashboard', path: 'dashboard', meta: { title: 'Dashboard' } }]
      }
    })
    writeAuthSession({
      token: 'token-123',
      userInfo: {
        id: 1,
        userName: 'operator',
        nickName: 'Operator'
      }
    })

    await bootstrapAuthSession()

    const authStore = useAuthStore()
    const menuStore = useMenuStore()
    expect(authStore.isAuthenticated).toBe(false)
    expect(menuStore.accessLoaded).toBe(false)
  })
})
