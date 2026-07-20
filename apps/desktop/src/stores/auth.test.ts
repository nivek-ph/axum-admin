import { afterEach, describe, expect, it } from 'vitest'

import { AUTH_STORAGE_KEY, readAuthSession } from './auth'

describe('persisted auth session', () => {
  afterEach(() => localStorage.clear())

  it('restores a complete session and discards incomplete or invalid data', () => {
    const valid = {
      accessToken: 'access',
      refreshToken: 'refresh',
      userInfo: { id: 1, userName: 'admin', nickName: 'Admin' },
    }
    localStorage.setItem(AUTH_STORAGE_KEY, JSON.stringify(valid))
    expect(readAuthSession()).toEqual(valid)
    localStorage.setItem(AUTH_STORAGE_KEY, JSON.stringify({ accessToken: 'access' }))
    expect(readAuthSession()).toEqual({ accessToken: '', refreshToken: '', userInfo: null })
    expect(localStorage.getItem(AUTH_STORAGE_KEY)).toBeNull()
    localStorage.setItem(AUTH_STORAGE_KEY, '{invalid')
    expect(readAuthSession()).toEqual({ accessToken: '', refreshToken: '', userInfo: null })
  })
})
