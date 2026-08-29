import type { AxiosAdapter } from 'axios'
import { afterEach, describe, expect, it } from 'vitest'

import {
  createStorage,
  deleteStorage,
  fetchStorages,
  setDefaultStorage,
  setStorageStatus,
  updateStorage,
  type StoragePayload,
} from './storages'
import { http } from './http'

const payload: StoragePayload = {
  name: 'Local storage',
  code: 'local_store',
  driver: 'local',
  root: './uploads',
  virtualHostStyle: false,
  enabled: true,
  sort: 10,
  description: '',
}

describe('storage API', () => {
  const originalAdapter = http.defaults.adapter

  afterEach(() => {
    http.defaults.adapter = originalAdapter
  })

  it('uses the storage management REST contract', async () => {
    const requests: Array<{ method?: string; url?: string; data?: unknown; params?: unknown }> = []
    http.defaults.adapter = (async (config) => {
      requests.push({ method: config.method, url: config.url, data: config.data, params: config.params })
      const data = config.method === 'get' ? { list: [] } : { id: 7, ...payload }
      return { data: { code: 'OK', message: 'ok', data }, status: 200, statusText: 'OK', headers: {}, config }
    }) as AxiosAdapter

    await fetchStorages({ keyword: 'local', driver: 'local' })
    await createStorage(payload)
    await updateStorage(7, payload)
    await setStorageStatus(7, false)
    await setDefaultStorage(7)
    await deleteStorage(7)

    expect(requests.map(({ method, url }) => `${method} ${url}`)).toEqual([
      'get /storages',
      'post /storages',
      'put /storages/7',
      'patch /storages/7/status',
      'put /storages/7/default',
      'delete /storages/7',
    ])
    expect(requests[0].params).toEqual({ keyword: 'local', driver: 'local' })
    expect(JSON.parse(String(requests[3].data))).toEqual({ enabled: false })
  })
})
