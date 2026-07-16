import { describe, expect, it } from 'vitest'

import { normalizeAuditEventListResponse } from './logs'

describe('audit events api adapter', () => {
  it('normalizes audit event payload', () => {
    const result = normalizeAuditEventListResponse({
      data: {
        list: [
          {
            id: 1,
            actorLabel: 'admin',
            action: 'user.assign_roles',
            resourceType: 'user',
            resourceId: '7',
            result: 'succeeded',
            sourceIp: '127.0.0.1',
            userAgent: 'vitest',
            changes: []
          }
        ],
        total: 1,
        page: 1,
        pageSize: 10
      }
    })

    expect(result.list).toHaveLength(1)
    expect(result.list[0].action).toBe('user.assign_roles')
    expect(result.total).toBe(1)
  })
})
