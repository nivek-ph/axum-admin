import { AxiosError, type AxiosAdapter, type InternalAxiosRequestConfig } from 'axios'
import { afterEach, describe, expect, it } from 'vitest'

import { uploadFile } from './files'
import { http } from './http'
import { useAuthStore } from '@/stores/auth'

describe('file upload adapter', () => {
  const originalAdapter = http.defaults.adapter
  afterEach(() => {
    http.defaults.adapter = originalAdapter
    useAuthStore.getState().clearSession()
    localStorage.clear()
  })

  it('uploads chunks with metadata, authorization, and offset progress', async () => {
    useAuthStore.getState().setSession({ accessToken: 'latest-token', refreshToken: 'refresh', userInfo: null })
    const progress: number[] = []
    let offset = 0
    http.defaults.adapter = (async (config) => {
      expect(config.headers.get('Authorization')).toBe('Bearer latest-token')
      let data: unknown
      if (config.url === '/files/uploads' && config.method === 'post') {
        expect(JSON.parse(config.data as string)).toMatchObject({
          name: 'evidence.txt',
          size: 4,
          tag: 'evidence',
          category: 'documents',
        })
        data = { id: 'session', offset: 0, totalSize: 4, chunkSize: 2 }
      } else if (config.url === '/files/uploads/session' && config.method === 'patch') {
        expect(Number(config.headers.get('Upload-Offset'))).toBe(offset)
        offset += (config.data as Blob).size
        data = { id: 'session', offset, totalSize: 4, chunkSize: 2 }
      } else if (config.url === '/files/uploads/session/complete' && config.method === 'post') data = {}
      else throw new Error(`Unexpected request: ${config.method} ${config.url}`)
      return { data: { code: 'OK', message: 'ok', data }, status: 200, statusText: 'OK', headers: {}, config }
    }) as AxiosAdapter

    await uploadFile(new File(['data'], 'evidence.txt'), { tag: 'evidence', category: 'documents' }, (value) => {
      progress.push(value)
    })
    expect(progress).toEqual([0, 50, 100])
  })

  it('refreshes once and retries an expired upload with the rotated bearer token', async () => {
    useAuthStore.getState().setSession({
      accessToken: 'old-token',
      refreshToken: 'old-refresh',
      userInfo: { id: 1, userName: 'admin', nickName: 'Admin' },
    })
    let starts = 0
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
      if (config.url === '/files/uploads' && config.method === 'post') {
        starts += 1
        if (starts === 1)
          throw new AxiosError('expired', '401', config, undefined, {
            data: { code: 'ACCESS_TOKEN_EXPIRED', message: 'expired' },
            status: 401,
            statusText: 'Unauthorized',
            headers: {},
            config,
          })
      }
      expect(config.headers.get('Authorization')).toBe('Bearer new-token')
      const data =
        config.url === '/files/uploads'
          ? { id: 'session', offset: 0, totalSize: 4, chunkSize: 4 }
          : config.url === '/files/uploads/session'
            ? { id: 'session', offset: 4, totalSize: 4, chunkSize: 4 }
            : {}
      return { data: { code: 'OK', message: 'ok', data }, status: 200, statusText: 'OK', headers: {}, config }
    }) as AxiosAdapter

    await uploadFile(new File(['data'], 'retry.txt'))
    expect({ starts, refreshes }).toEqual({ starts: 2, refreshes: 1 })
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

  it('keeps the resumable session when its status lookup fails transiently', async () => {
    const file = new File(['data'], 'resume.txt', { lastModified: 123 })
    const resumeKey = `file-upload:${JSON.stringify([file.name, file.size, file.lastModified, '', ''])}`
    localStorage.setItem(resumeKey, 'saved-session')
    const requests: string[] = []
    http.defaults.adapter = (async (config: InternalAxiosRequestConfig) => {
      requests.push(`${config.method} ${config.url}`)
      throw new AxiosError('failed', '500', config, undefined, {
        data: { code: 'INTERNAL_SERVER_ERROR', message: 'temporary failure' },
        status: 500,
        statusText: 'Error',
        headers: {},
        config,
      })
    }) as AxiosAdapter

    await expect(uploadFile(file)).rejects.toThrow('temporary failure')
    expect(localStorage.getItem(resumeKey)).toBe('saved-session')
    expect(requests).toEqual(['get /files/uploads/saved-session'])
  })

  it('does not resume a session created with different metadata', async () => {
    const file = new File(['data'], 'resume.txt', { lastModified: 123 })
    const previousKey = `file-upload:${JSON.stringify([
      file.name,
      file.size,
      file.lastModified,
      'old-tag',
      'old-category',
    ])}`
    localStorage.setItem(previousKey, 'old-session')
    const requests: string[] = []
    http.defaults.adapter = (async (config: InternalAxiosRequestConfig) => {
      requests.push(`${config.method} ${config.url}`)
      const data =
        config.url === '/files/uploads'
          ? { id: 'new-session', offset: 0, totalSize: 4, chunkSize: 4 }
          : config.url === '/files/uploads/new-session'
            ? { id: 'new-session', offset: 4, totalSize: 4, chunkSize: 4 }
            : {}
      return { data: { code: 'OK', message: 'ok', data }, status: 200, statusText: 'OK', headers: {}, config }
    }) as AxiosAdapter

    await uploadFile(file, { tag: 'new-tag', category: 'new-category' })

    expect(requests).toEqual([
      'post /files/uploads',
      'patch /files/uploads/new-session',
      'post /files/uploads/new-session/complete',
    ])
    expect(localStorage.getItem(previousKey)).toBe('old-session')
  })
})
