import { describe, expect, it } from 'vitest'

import {
  buildRoleDeptPayload,
  buildRolePermissionPayload,
  buildRoleUsersPayload,
  normalizeRoleIds,
  normalizeRoleList
} from './roles'

describe('system role api adapter', () => {
  it('normalizes role list payload', () => {
    const result = normalizeRoleList({
      data: {
        list: [
          {
            id: 1,
            code: 'super_admin',
            name: 'Super Admin',
            status: 'enabled',
            sort: 0,
            data_scope: 'all',
            is_system: true
          }
        ]
      }
    })

    expect(result).toHaveLength(1)
    expect(result[0].code).toBe('super_admin')
  })

  it('normalizes role menu and department id payloads', () => {
    expect(normalizeRoleIds({ data: { menuIds: [1, 2] } }, 'menuIds')).toEqual([1, 2])
    expect(normalizeRoleIds({ data: { deptIds: [3] } }, 'deptIds')).toEqual([3])
  })

  it('builds camelCase assignment payloads', () => {
    expect(buildRolePermissionPayload([2, 1])).toEqual({ menuIds: [1, 2] })
    expect(buildRoleDeptPayload([9, 3])).toEqual({ deptIds: [3, 9] })
    expect(buildRoleUsersPayload([4, 2, 4])).toEqual({ userIds: [2, 4] })
  })
})
