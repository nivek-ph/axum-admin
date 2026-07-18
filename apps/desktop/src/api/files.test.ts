import {
  AxiosError,
  type AxiosAdapter,
  type AxiosProgressEvent,
} from 'axios'
import { createPinia, setActivePinia } from 'pinia'
import { afterEach, beforeEach, describe, expect, it } from 'vitest'

import { useAuthStore } from '@/stores/auth'

import { http } from './http'
import { adapterResponse, rejectEnvelope } from './httpTestUtils'
import { normalizeFileListResponse, uploadFile } from './files'

describe('files api adapter', () => {
  const originalAdapter = http.defaults.adapter

  beforeEach(() => {
    setActivePinia(createPinia())
    localStorage.clear()
    window.location.hash = '#/files'
  })

  afterEach(() => {
    http.defaults.adapter = originalAdapter
  })

  function establishSession(accessToken = 'access-latest') {
    useAuthStore().setSession(accessToken, 'refresh-token', {
      id: 1,
      userName: 'admin',
      nickName: 'Admin',
    })
  }

  it('normalizes file list payload', () => {
    const result = normalizeFileListResponse({
      code: 'OK',
      message: 'ok',
      data: {
        list: [{
          id: 1,
          name: 'logo.png',
          url: '/uploads/logo.png',
          ext: 'png',
          tag: 'brand',
          category: 'images',
          updatedAt: '2026-07-13T00:00:00',
        }],
        total: 1,
        page: 1,
        pageSize: 10,
      },
    })

    expect(result.list).toHaveLength(1)
  })

  it('keeps upload metadata, form data, latest access token, and progress behavior', async () => {
    establishSession()
    const progress: number[] = []
    const file = new File(['hello'], 'hello.txt', { type: 'text/plain' })
    http.defaults.adapter = (async (config) => {
      expect(config.url).toBe('/files/upload')
      expect(config.params).toEqual({ tag: 'evidence', category: 'documents' })
      expect(config.headers.get('Authorization')).toBe('Bearer access-latest')
      expect(config.data).toBeInstanceOf(FormData)
      expect((config.data as FormData).get('file')).toBe(file)
      config.onUploadProgress?.({ loaded: 5, total: 8 } as AxiosProgressEvent)
      return adapterResponse(config, { code: 'OK', message: 'uploaded', data: { id: 1 } })
    }) as AxiosAdapter

    const result = await uploadFile(
      file,
      { tag: 'evidence', category: 'documents' },
      (value) => progress.push(value),
    )

    expect(progress).toEqual([63])
    expect(result).toEqual({ code: 'OK', message: 'uploaded', data: { id: 1 } })
  })

  it('refreshes once and retries once with the new Authorization header', async () => {
    establishSession('access-old')
    const file = new File(['hello'], 'hello.txt')
    let uploadCalls = 0
    let refreshCalls = 0
    http.defaults.adapter = (async (config) => {
      if (config.url === '/auth/refresh') {
        refreshCalls += 1
        return adapterResponse(config, {
          code: 'OK',
          message: 'ok',
          data: { accessToken: 'access-new', refreshToken: 'refresh-new' },
        })
      }
      uploadCalls += 1
      if (uploadCalls === 1) return rejectEnvelope(config, 'ACCESS_TOKEN_EXPIRED')
      expect(config.headers.get('Authorization')).toBe('Bearer access-new')
      expect((config.data as FormData).get('file')).toBe(file)
      return adapterResponse(config, { code: 'OK', message: 'uploaded', data: { id: 2 } })
    }) as AxiosAdapter

    await uploadFile(file)

    expect(refreshCalls).toBe(1)
    expect(uploadCalls).toBe(2)
  })

  it('does not create a retry loop after repeated access expiration', async () => {
    establishSession('access-old')
    let uploadCalls = 0
    let refreshCalls = 0
    http.defaults.adapter = (async (config) => {
      if (config.url === '/auth/refresh') {
        refreshCalls += 1
        return adapterResponse(config, {
          code: 'OK',
          message: 'ok',
          data: { accessToken: 'access-new', refreshToken: 'refresh-new' },
        })
      }
      uploadCalls += 1
      return rejectEnvelope(config, 'ACCESS_TOKEN_EXPIRED')
    }) as AxiosAdapter

    await expect(uploadFile(new File(['hello'], 'hello.txt'))).rejects.toBeInstanceOf(Error)

    expect(refreshCalls).toBe(1)
    expect(uploadCalls).toBe(2)
  })

  it('does not retry the upload when refresh fails', async () => {
    establishSession('access-old')
    let uploadCalls = 0
    let refreshCalls = 0
    http.defaults.adapter = (async (config) => {
      if (config.url === '/auth/refresh') {
        refreshCalls += 1
        throw new AxiosError('refresh network failed', 'ERR_NETWORK', config)
      }
      uploadCalls += 1
      return rejectEnvelope(config, 'ACCESS_TOKEN_EXPIRED')
    }) as AxiosAdapter

    await expect(uploadFile(new File(['hello'], 'hello.txt'))).rejects.toBeInstanceOf(Error)

    expect(refreshCalls).toBe(1)
    expect(uploadCalls).toBe(1)
  })

  it('does not retry other authentication or network failures', async () => {
    for (const failure of ['SESSION_INVALID', 'network'] as const) {
      establishSession()
      let uploadCalls = 0
      let refreshCalls = 0
      http.defaults.adapter = (async (config) => {
        if (config.url === '/auth/refresh') refreshCalls += 1
        uploadCalls += 1
        if (failure === 'network') {
          throw new AxiosError('network failed', 'ERR_NETWORK', config)
        }
        return rejectEnvelope(config, failure)
      }) as AxiosAdapter

      await expect(uploadFile(new File(['hello'], 'hello.txt'))).rejects.toBeInstanceOf(Error)
      expect(refreshCalls).toBe(0)
      expect(uploadCalls).toBe(1)
    }
  })
})
