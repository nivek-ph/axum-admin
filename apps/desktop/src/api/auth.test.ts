import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it, vi } from 'vitest'

import { useAuthStore } from '@/stores/auth'

const httpApi = vi.hoisted(() => ({
  post: vi.fn(),
}))

vi.mock('./http', () => ({
  http: {
    post: httpApi.post,
  },
}))

import { logout } from './auth'

describe('auth api adapter', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    httpApi.post.mockReset()
    httpApi.post.mockResolvedValue({ code: 'OK', message: 'signed out' })
  })

  it('sends logout with the current access token', async () => {
    useAuthStore().setSession('access-token', 'refresh-token', {
      id: 1,
      userName: 'admin',
      nickName: 'Admin',
    })

    await logout()

    expect(httpApi.post).toHaveBeenCalledWith('/auth/logout', undefined, {
      headers: {
        Authorization: 'Bearer access-token',
      },
    })
  })
})
