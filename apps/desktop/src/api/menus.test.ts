import { describe, expect, it } from 'vitest'

import {
  normalizeMenuListResponse,
  normalizeMenuRoleSelection
} from './menus'

describe('menu api adapter', () => {
  it('normalizes backend menu tree payload', () => {
    const result = normalizeMenuListResponse({
      data: [
        {
          ID: 1,
          name: 'dashboard',
          meta: { title: 'Dashboard' },
          children: [{ ID: 2, name: 'users', meta: { title: 'User' } }]
        }
      ]
    })

    expect(result).toHaveLength(1)
    expect(result[0].children).toHaveLength(1)
  })

  it('normalizes assigned role ids', () => {
    const result = normalizeMenuRoleSelection({
      data: {
        roleIds: [1]
      }
    })

    expect(result.roleIds).toEqual([1])
  })

})
