import { AxiosError, type AxiosAdapter } from 'axios'
import { afterEach, describe, expect, it } from 'vitest'

import { changeOwnPassword, updateOwnProfile } from './users'
import { http } from './http'
import { useAuthStore } from '@/stores/auth'

describe('changeOwnPassword', () => {
  const originalAdapter = http.defaults.adapter

  afterEach(() => {
    http.defaults.adapter = originalAdapter
    useAuthStore.getState().clearSession()
  })

  it('puts the current and new password to /users/me/password with the bearer token', async () => {
    useAuthStore.getState().setSession({ accessToken: 'token', refreshToken: 'refresh', userInfo: null })
    http.defaults.adapter = (async (config) => {
      expect(config.method?.toLowerCase()).toBe('put')
      expect(config.url).toBe('/users/me/password')
      expect(config.headers.get('Authorization')).toBe('Bearer token')
      expect(config.data).toBe(JSON.stringify({ password: 'old-password', newPassword: 'new-password' }))
      return {
        data: { code: 'OK', message: 'updated', data: null },
        status: 200,
        statusText: 'OK',
        headers: {},
        config,
      }
    }) as AxiosAdapter

    const response = await changeOwnPassword({ password: 'old-password', newPassword: 'new-password' })
    expect(response.code).toBe('OK')
  })

  it('surfaces invalid current password errors', async () => {
    useAuthStore.getState().setSession({ accessToken: 'token', refreshToken: 'refresh', userInfo: null })
    http.defaults.adapter = (async (config) => {
      throw new AxiosError('invalid', '400', config, undefined, {
        data: { code: 'INVALID_PASSWORD', message: 'invalid password' },
        status: 400,
        statusText: 'Bad Request',
        headers: {},
        config,
      })
    }) as AxiosAdapter

    await expect(changeOwnPassword({ password: 'wrong', newPassword: 'new-password' })).rejects.toThrow(
      'invalid password',
    )
  })
})

describe('updateOwnProfile', () => {
  const originalAdapter = http.defaults.adapter

  afterEach(() => {
    http.defaults.adapter = originalAdapter
    useAuthStore.getState().clearSession()
  })

  it('puts nickname phone and email to /users/me with the bearer token', async () => {
    useAuthStore.getState().setSession({ accessToken: 'token', refreshToken: 'refresh', userInfo: null })
    http.defaults.adapter = (async (config) => {
      expect(config.method?.toLowerCase()).toBe('put')
      expect(config.url).toBe('/users/me')
      expect(config.headers.get('Authorization')).toBe('Bearer token')
      expect(config.data).toBe(JSON.stringify({ nickName: 'Ada', phone: '13800000000', email: 'ada@example.com' }))
      return {
        data: { code: 'OK', message: 'updated', data: null },
        status: 200,
        statusText: 'OK',
        headers: {},
        config,
      }
    }) as AxiosAdapter

    const response = await updateOwnProfile({ nickName: 'Ada', phone: '13800000000', email: 'ada@example.com' })
    expect(response.code).toBe('OK')
  })
})
