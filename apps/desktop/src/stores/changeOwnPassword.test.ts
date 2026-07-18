import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it, vi } from 'vitest'

import { useAuthStore } from './auth'
import { useMenuStore } from './menu'

const usersApi = vi.hoisted(() => ({
  changeOwnPassword: vi.fn(),
}))

vi.mock('@/api/users', () => ({
  changeOwnPassword: usersApi.changeOwnPassword,
}))

import { changeOwnPasswordAndSignOut } from './changeOwnPassword'

describe('changeOwnPasswordAndSignOut', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    usersApi.changeOwnPassword.mockReset()
  })

  function establishSession() {
    useAuthStore().setSession('access-token', 'refresh-token', {
      id: 1,
      userName: 'admin',
      nickName: 'Admin',
    })
    useMenuStore().setAuthorizedMenus(
      [{ name: 'dashboard', path: '/dashboard', label: 'Dashboard' }],
      true,
    )
  }

  it('clears local access and returns to login after a successful change', async () => {
    establishSession()
    usersApi.changeOwnPassword.mockResolvedValue({ code: 'OK' })
    const navigateToLogin = vi.fn()

    await changeOwnPasswordAndSignOut(
      { password: 'old-password', newPassword: 'new-password' },
      navigateToLogin,
    )

    expect(useAuthStore().isAuthenticated).toBe(false)
    expect(useMenuStore().accessLoaded).toBe(false)
    expect(navigateToLogin).toHaveBeenCalledOnce()
  })

  it('keeps the current session when the password change fails', async () => {
    establishSession()
    usersApi.changeOwnPassword.mockRejectedValue(new Error('request failed'))
    const navigateToLogin = vi.fn()

    await expect(
      changeOwnPasswordAndSignOut(
        { password: 'wrong-password', newPassword: 'new-password' },
        navigateToLogin,
      ),
    ).rejects.toThrow('request failed')

    expect(useAuthStore().isAuthenticated).toBe(true)
    expect(useMenuStore().accessLoaded).toBe(true)
    expect(navigateToLogin).not.toHaveBeenCalled()
  })
})
