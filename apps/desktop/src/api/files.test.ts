import { describe, expect, it } from 'vitest'

import { normalizeFileListResponse } from './files'

describe('files api adapter', () => {
  it('normalizes file list payload', () => {
    const result = normalizeFileListResponse({
      data: {
        list: [{
          id: 1,
          name: 'logo.png',
          url: '/uploads/logo.png',
          ext: 'png',
          tag: 'brand',
          category: 'images',
          updatedAt: '2026-07-13T00:00:00'
        }],
        total: 1,
        page: 1,
        pageSize: 10
      }
    })

    expect(result.list).toHaveLength(1)
  })
})
