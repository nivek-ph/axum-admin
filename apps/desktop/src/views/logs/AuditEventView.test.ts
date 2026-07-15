import { flushPromises, mount } from '@vue/test-utils'
import { UiComponents } from '@/components/ui'
import { beforeEach, describe, expect, it, vi } from 'vitest'

const mocks = vi.hoisted(() => ({
  fetchAuditEvents: vi.fn(),
  fetchAuditEvent: vi.fn()
}))

vi.mock('@/api/logs', () => mocks)

vi.mock('@/ui/feedback', () => ({
  ElMessage: {
    error: vi.fn()
  }
}))

import AuditEventView from './AuditEventView.vue'

describe('AuditEventView', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    mocks.fetchAuditEvents.mockResolvedValue({
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
    })
  })

  it('renders the unified audit event list', async () => {
    const wrapper = mount(AuditEventView, {
      global: {
        plugins: [UiComponents]
      }
    })

    await flushPromises()
    expect(mocks.fetchAuditEvents).toHaveBeenCalledOnce()
    expect(wrapper.text()).toContain('Audit events')
    expect(wrapper.text()).toContain('user.assign_roles')
    expect(wrapper.text()).not.toContain('Operation logs')
  })
})
