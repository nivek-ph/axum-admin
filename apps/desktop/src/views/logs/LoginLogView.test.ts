import { mount } from '@vue/test-utils'
import { UiComponents } from '@/components/ui'
import { beforeEach, describe, expect, it, vi } from 'vitest'

const mocks = vi.hoisted(() => ({
  fetchLoginLogs: vi.fn()
}))

vi.mock('@/api/logs', () => ({
  fetchLoginLogs: mocks.fetchLoginLogs
}))

vi.mock('@/ui/feedback', () => ({
  ElMessage: {
    success: vi.fn(),
    error: vi.fn()
  }
}))

import LoginLogView from './LoginLogView.vue'

describe('LoginLogView', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    mocks.fetchLoginLogs.mockResolvedValue({
      list: [],
      total: 0,
      page: 1,
      pageSize: 10
    })
  })

  it('renders login log heading', async () => {
    const wrapper = mount(LoginLogView, {
      global: {
        plugins: [UiComponents]
      }
    })

    await Promise.resolve()
    expect(wrapper.text()).toContain('Login logs')
  })
})
