import { AxiosError, type AxiosAdapter, type AxiosProgressEvent, type InternalAxiosRequestConfig } from 'axios'
import { afterEach, describe, expect, it } from 'vitest'

import { uploadFile } from './files'
import { http } from './http'
import { useAuthStore } from '@/stores/auth'

describe('file upload adapter', () => {
  const originalAdapter = http.defaults.adapter
  afterEach(() => {
    http.defaults.adapter = originalAdapter
    useAuthStore.getState().clearSession()
  })

  it('uses form data, current bearer token, metadata, and reports callback progress', async () => {
    useAuthStore.getState().setSession({ accessToken: 'latest-token', refreshToken: 'refresh', userInfo: null })
    let progress = 0
    http.defaults.adapter = (async (config) => {
      expect(config.headers.get('Authorization')).toBe('Bearer latest-token')
      expect(config.params).toMatchObject({ tag: 'evidence', category: 'documents' })
      expect((config.data as FormData).get('file')).toBeInstanceOf(File)
      config.onUploadProgress?.({ loaded: 3, total: 4 } as AxiosProgressEvent)
      return { data: { code: 'OK', message: 'ok', data: {} }, status: 200, statusText: 'OK', headers: {}, config }
    }) as AxiosAdapter

    await uploadFile(new File(['data'], 'evidence.txt'), { tag: 'evidence', category: 'documents' }, (value) => {
      progress = value
    })
    expect(progress).toBe(75)
  })

  it('refreshes once and retries an expired upload with the rotated bearer token', async () => {
    useAuthStore.getState().setSession({
      accessToken: 'old-token',
      refreshToken: 'old-refresh',
      userInfo: { id: 1, userName: 'admin', nickName: 'Admin' },
    })
    let uploads = 0
    let refreshes = 0
    http.defaults.adapter = (async (config) => {
      if (config.url === '/auth/refresh') {
        refreshes += 1
        return {
          data: { code: 'OK', message: 'ok', data: { accessToken: 'new-token', refreshToken: 'new-refresh' } },
          status: 200,
          statusText: 'OK',
          headers: {},
          config,
        }
      }
      uploads += 1
      if (uploads === 1)
        throw new AxiosError('expired', '401', config, undefined, {
          data: { code: 'ACCESS_TOKEN_EXPIRED', message: 'expired' },
          status: 401,
          statusText: 'Unauthorized',
          headers: {},
          config,
        })
      expect(config.headers.get('Authorization')).toBe('Bearer new-token')
      expect((config.data as FormData).get('file')).toBeInstanceOf(File)
      return { data: { code: 'OK', message: 'ok', data: {} }, status: 200, statusText: 'OK', headers: {}, config }
    }) as AxiosAdapter

    await uploadFile(new File(['data'], 'retry.txt'))
    expect({ uploads, refreshes }).toEqual({ uploads: 2, refreshes: 1 })
  })

  it('does not refresh or retry a non-auth upload failure', async () => {
    useAuthStore.getState().setSession({
      accessToken: 'token',
      refreshToken: 'refresh',
      userInfo: { id: 1, userName: 'admin', nickName: 'Admin' },
    })
    let attempts = 0
    http.defaults.adapter = (async (config: InternalAxiosRequestConfig) => {
      attempts += 1
      throw new AxiosError('failed', '500', config, undefined, {
        data: { code: 'FILE_STORAGE_ERROR', message: 'failed' },
        status: 500,
        statusText: 'Error',
        headers: {},
        config,
      })
    }) as AxiosAdapter
    await expect(uploadFile(new File(['data'], 'failed.txt'))).rejects.toThrow('failed')
    expect(attempts).toBe(1)
  })
})
