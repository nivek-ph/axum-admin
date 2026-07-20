import { AxiosError, type AxiosAdapter, type InternalAxiosRequestConfig } from 'axios'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

import { bearerAuthorization } from './core'
import { getApiErrorMessage, http } from './http'
import { useAuthStore } from '@/stores/auth'
import { useMenuStore } from '@/stores/menu'

function response(config: InternalAxiosRequestConfig, data: unknown) {
  return Promise.resolve({ data, status: 200, statusText: 'OK', headers: {}, config })
}

function rejectEnvelope(config: InternalAxiosRequestConfig, code: string, status = 401) {
  return Promise.reject(
    new AxiosError('request failed', String(status), config, undefined, {
      data: { code, message: code },
      status,
      statusText: 'Error',
      headers: {},
      config,
    }),
  )
}

describe('authenticated HTTP requests', () => {
  const originalAdapter = http.defaults.adapter

  beforeEach(() => {
    useAuthStore.getState().clearSession()
    useMenuStore.getState().resetAccess()
  })

  afterEach(() => {
    http.defaults.adapter = originalAdapter
  })

  it('shares one token refresh across concurrent expired requests', async () => {
    useAuthStore.getState().setSession({
      accessToken: 'access-old',
      refreshToken: 'refresh-old',
      userInfo: { id: 1, userName: 'admin', nickName: 'Admin' },
    })
    let refreshCalls = 0
    let releaseRefresh!: () => void
    const refreshBarrier = new Promise<void>((resolve) => {
      releaseRefresh = resolve
    })
    const attempts = new Map<string, number>()
    const retryHeaders: string[] = []

    http.defaults.adapter = (async (config) => {
      if (config.url === '/auth/refresh') {
        refreshCalls += 1
        await refreshBarrier
        return response(config, {
          code: 'OK',
          message: 'ok',
          data: { accessToken: 'access-new', refreshToken: 'refresh-new' },
        })
      }
      const attempt = (attempts.get(config.url ?? '') ?? 0) + 1
      attempts.set(config.url ?? '', attempt)
      if (attempt === 1) return rejectEnvelope(config, 'ACCESS_TOKEN_EXPIRED')
      retryHeaders.push(String(config.headers.get('Authorization')))
      return response(config, { code: 'OK', message: 'ok', data: null })
    }) as AxiosAdapter

    const requests = [
      http.get('/first', { headers: { Authorization: bearerAuthorization('access-old') } }),
      http.get('/second', { headers: { Authorization: bearerAuthorization('access-old') } }),
    ]
    await vi.waitFor(() => expect(refreshCalls).toBe(1))
    releaseRefresh()
    await Promise.all(requests)

    expect(retryHeaders).toEqual(['Bearer access-new', 'Bearer access-new'])
    expect(useAuthStore.getState()).toMatchObject({ accessToken: 'access-new', refreshToken: 'refresh-new' })
  })
})

describe('API error messages', () => {
  it('uses the contextual fallback when the server is unreachable', () => {
    expect(getApiErrorMessage(new AxiosError('Network Error'), 'Sign in failed')).toBe('Sign in failed')
  })
})
