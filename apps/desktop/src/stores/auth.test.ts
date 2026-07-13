import { createPinia, setActivePinia } from 'pinia'
import { afterEach, describe, expect, it } from 'vitest'
import { useAuthStore } from './auth'
import { clearAuthSession, readAuthSession } from './authStorage'

describe('auth store', () => {
  afterEach(() => {
    clearAuthSession()
  })

  it('starts logged out', () => {
    setActivePinia(createPinia())
    const store = useAuthStore()

    expect(store.token).toBe('')
    expect(store.isAuthenticated).toBe(false)
  })

  it('stores token and user info after login state is set', () => {
    setActivePinia(createPinia())
    const store = useAuthStore()

    store.setSession('token-123', {
      id: 1,
      userName: 'admin',
      nickName: 'System Administrator'
    })

    expect(store.token).toBe('token-123')
    expect(store.isAuthenticated).toBe(true)
    expect(store.userInfo?.userName).toBe('admin')
    expect(readAuthSession().token).toBe('token-123')
  })

  it('clears persisted session on logout', () => {
    setActivePinia(createPinia())
    const store = useAuthStore()

    store.setSession('token-123', {
      id: 1,
      userName: 'admin',
      nickName: 'System Administrator'
    })
    store.clearToken()

    expect(store.isAuthenticated).toBe(false)
    expect(readAuthSession().token).toBe('')
  })

  it('checks permission codes through one helper', () => {
    setActivePinia(createPinia())
    const store = useAuthStore()

    store.setSession('token-123', {
      id: 2,
      userName: 'operator',
      nickName: 'Operator',
      roles: [{ id: 2, code: 'operator', name: 'Operator' }],
      roleIds: [2],
      permissions: ['system:user:list']
    })

    expect(store.can('system:user:list')).toBe(true)
    expect(store.can('system:user:delete')).toBe(false)
  })

  it('builds the visible role label from all assigned roles', () => {
    setActivePinia(createPinia())
    const store = useAuthStore()

    store.setSession('token-123', {
      id: 1,
      userName: 'admin',
      nickName: 'Admin',
      roles: [
        { id: 1, code: 'super_admin', name: 'Super Admin' },
        { id: 2, code: 'operator', name: 'Operator' }
      ]
    })

    expect(store.roleLabel).toBe('Super Admin / Operator')
  })

  it('allows super admin by role code', () => {
    setActivePinia(createPinia())
    const store = useAuthStore()

    store.setSession('token-123', {
      id: 1,
      userName: 'admin',
      nickName: 'Admin',
      roles: [{ id: 1, code: 'super_admin', name: 'Super Admin' }],
      roleIds: [1],
      permissions: []
    })

    expect(store.isSuperAdmin).toBe(true)
    expect(store.can('system:anything:anything')).toBe(true)
  })
})
