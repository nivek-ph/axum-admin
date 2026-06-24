import { describe, expect, it } from 'vitest'

import {
  normalizeAuthorityMenuSelection,
  normalizeMenuRoleMatrixResponse,
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
        authorityIds: [888],
        defaultRouterAuthorityIds: [888]
      }
    })

    expect(result.authorityIds).toEqual([888])
    expect(result.defaultRouterAuthorityIds).toEqual([888])
  })

  it('normalizes menu ids assigned to one role', () => {
    const result = normalizeAuthorityMenuSelection({
      data: {
        menus: [
          { menuId: 1, parentId: 0 },
          { ID: 2, parentId: 1 }
        ]
      }
    })

    expect(result).toEqual([1, 2])
  })

  it('normalizes menu role matrix payload', () => {
    const result = normalizeMenuRoleMatrixResponse({
      data: {
        items: [
          { menuId: 1, authorityIds: [1] },
          { menuId: 2, authorityIds: [1, 888] }
        ]
      }
    })

    expect(result).toEqual({
      1: [1],
      2: [1, 888]
    })
  })
})
