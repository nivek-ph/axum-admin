import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'

import { http } from './http'
import {
  createDictionaryDetail,
  deleteDictionaryDetail,
  fetchDictionaryDetails,
  normalizeDictionaryDetailTreeResponse,
  normalizeDictionaryListResponse,
  updateDictionaryDetail,
} from './dictionaries'

describe('dictionaries api adapter', () => {
  beforeEach(() => {
    vi.restoreAllMocks()
    setActivePinia(createPinia())
  })

  it('normalizes dictionary list payload', () => {
    const result = normalizeDictionaryListResponse({
      data: [{ id: 1, name: 'Status', type: 'status', desc: '' }]
    })

    expect(result).toHaveLength(1)
  })

  it('normalizes dictionary detail tree payload', () => {
    const result = normalizeDictionaryDetailTreeResponse({
      data: {
        list: [{ id: 1, label: 'Enabled', value: 'enabled', children: [] }]
      }
    })

    expect(result).toHaveLength(1)
  })

  it('uses dictionary-scoped tree endpoints for detail operations', async () => {
    const get = vi.spyOn(http, 'get').mockResolvedValue({ data: { list: [] } })
    const post = vi.spyOn(http, 'post').mockResolvedValue({ code: 'OK' })
    const put = vi.spyOn(http, 'put').mockResolvedValue({ code: 'OK' })
    const remove = vi.spyOn(http, 'delete').mockResolvedValue({ code: 'OK' })
    const detail = {
      id: 9,
      label: 'Enabled',
      value: 'enabled',
      extend: '',
      status: true,
      sort: 0,
      sysDictionaryId: 7,
      parentId: null,
    }

    await fetchDictionaryDetails(7)
    await createDictionaryDetail(detail)
    await updateDictionaryDetail(detail)
    await deleteDictionaryDetail(7, 9)

    expect(get).toHaveBeenCalledWith('/dictionaries/7/tree', expect.any(Object))
    expect(post).toHaveBeenCalledWith(
      '/dictionaries/7/tree',
      expect.objectContaining({ id: 9, sysDictionaryId: 7 }),
      expect.any(Object)
    )
    expect(put).toHaveBeenCalledWith(
      '/dictionaries/7/tree/9',
      expect.objectContaining({ id: 9, sysDictionaryId: 7 }),
      expect.any(Object)
    )
    expect(remove).toHaveBeenCalledWith('/dictionaries/7/tree/9', expect.any(Object))
  })
})
