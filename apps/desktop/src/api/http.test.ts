import axios, { AxiosError, type AxiosAdapter, type InternalAxiosRequestConfig } from 'axios'
import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it, vi } from 'vitest'

import { useAuthStore } from '@/stores/auth'
import { useMenuStore } from '@/stores/menu'
import { ElMessage } from '@/ui/feedback'

import { withAuthHeaders } from './core'
import { http } from './http'

function response(config: InternalAxiosRequestConfig, data: unknown, status = 200) {
  return Promise.resolve({
    data,
    status,
    statusText: status === 200 ? 'OK' : 'Unauthorized',
    headers: {},
    config,
  })
}

function rejectEnvelope(config: InternalAxiosRequestConfig, code: string, status = 401) {
  const envelope = { code, message: 'session expired', data: null }
  return Promise.reject(
    new AxiosError('request failed', 'ERR_BAD_REQUEST', config, undefined, {
      data: envelope,
      status,
      statusText: 'Unauthorized',
      headers: {},
      config,
    })
  )
}

describe('auth response interceptor', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    localStorage.clear()
    window.location.hash = '#/dashboard'
  })

  it('shares one refresh and replaces Authorization before retrying concurrent requests', async () => {
    const auth = useAuthStore()
    auth.setSession('access-one', 'refresh-one', {
      id: 1,
      userName: 'admin',
      nickName: 'Admin',
    })
    let refreshCalls = 0
    const attempts = new Map<string, number>()
    let releaseRefresh!: () => void
    const refreshBarrier = new Promise<void>((resolve) => {
      releaseRefresh = resolve
    })
    const retryHeaders: string[] = []
    http.defaults.adapter = (async (config) => {
      if (config.url === '/auth/refresh') {
        refreshCalls += 1
        await refreshBarrier
        return response(config, {
          code: 'OK',
          message: 'ok',
          data: { accessToken: 'access-two', refreshToken: 'refresh-two' },
        })
      }
      const attempt = (attempts.get(config.url || '') || 0) + 1
      attempts.set(config.url || '', attempt)
      if (attempt === 1) return rejectEnvelope(config, 'ACCESS_TOKEN_EXPIRED')
      retryHeaders.push(String(config.headers.get('Authorization')))
      return response(config, { code: 'OK', message: 'ok', data: config.url })
    }) as AxiosAdapter

    const requests = [
      http.get('/first', withAuthHeaders()),
      http.get('/second', withAuthHeaders()),
    ]
    await vi.waitFor(() => expect(refreshCalls).toBe(1))
    releaseRefresh()
    await Promise.all(requests)

    expect(refreshCalls).toBe(1)
    expect(retryHeaders).toEqual(['Bearer access-two', 'Bearer access-two'])
    expect(auth.accessToken).toBe('access-two')
    expect(auth.refreshToken).toBe('refresh-two')
  })

  it('refreshes an expired logout once and retries logout once', async () => {
    useAuthStore().setSession('access-one', 'refresh-one', {
      id: 1,
      userName: 'admin',
      nickName: 'Admin',
    })
    let logoutCalls = 0
    let refreshCalls = 0
    http.defaults.adapter = (async (config) => {
      if (config.url === '/auth/refresh') {
        refreshCalls += 1
        return response(config, {
          code: 'OK',
          message: 'ok',
          data: { accessToken: 'access-two', refreshToken: 'refresh-two' },
        })
      }
      logoutCalls += 1
      if (logoutCalls === 1) return rejectEnvelope(config, 'ACCESS_TOKEN_EXPIRED')
      expect(config.headers.get('Authorization')).toBe('Bearer access-two')
      return response(config, { code: 'OK', message: 'ok', data: null })
    }) as AxiosAdapter

    await http.post('/auth/logout', undefined, withAuthHeaders())

    expect(refreshCalls).toBe(1)
    expect(logoutCalls).toBe(2)
  })

  it('never refreshes a retried request twice and clears local access state', async () => {
    const auth = useAuthStore()
    const menu = useMenuStore()
    auth.setSession('access-one', 'refresh-one', {
      id: 1,
      userName: 'admin',
      nickName: 'Admin',
    })
    menu.setAuthorizedMenus([], false)
    const warning = vi.spyOn(ElMessage, 'warning')
    let protectedCalls = 0
    let refreshCalls = 0
    http.defaults.adapter = (async (config) => {
      if (config.url === '/auth/refresh') {
        refreshCalls += 1
        return response(config, {
          code: 'OK',
          message: 'ok',
          data: { accessToken: 'access-two', refreshToken: 'refresh-two' },
        })
      }
      protectedCalls += 1
      return rejectEnvelope(config, 'ACCESS_TOKEN_EXPIRED')
    }) as AxiosAdapter

    await expect(http.get('/protected', withAuthHeaders())).rejects.toBeInstanceOf(Error)

    expect(refreshCalls).toBe(1)
    expect(protectedCalls).toBe(2)
    expect(auth.isAuthenticated).toBe(false)
    expect(menu.accessLoaded).toBe(false)
    expect(warning).toHaveBeenCalledOnce()
  })

  it('does not refresh other authentication failures', async () => {
    const auth = useAuthStore()
    auth.setSession('access-one', 'refresh-one', {
      id: 1,
      userName: 'admin',
      nickName: 'Admin',
    })
    let refreshCalls = 0
    http.defaults.adapter = (async (config) => {
      if (config.url === '/auth/refresh') refreshCalls += 1
      return rejectEnvelope(config, 'SESSION_INVALID')
    }) as AxiosAdapter

    await expect(http.get('/protected', withAuthHeaders())).rejects.toBeInstanceOf(Error)

    expect(refreshCalls).toBe(0)
    expect(auth.isAuthenticated).toBe(false)
  })

  it('clears a disabled user session without refreshing', async () => {
    const auth = useAuthStore()
    auth.setSession('access-one', 'refresh-one', {
      id: 1,
      userName: 'admin',
      nickName: 'Admin',
    })
    let refreshCalls = 0
    http.defaults.adapter = (async (config) => {
      if (config.url === '/auth/refresh') refreshCalls += 1
      return rejectEnvelope(config, 'USER_DISABLED', 403)
    }) as AxiosAdapter

    await expect(http.get('/protected', withAuthHeaders())).rejects.toBeInstanceOf(Error)

    expect(refreshCalls).toBe(0)
    expect(auth.isAuthenticated).toBe(false)
  })
})
