import { flushPromises, mount } from '@vue/test-utils'
import { UiComponents } from '@/components/ui'
import { beforeEach, describe, expect, it, vi } from 'vitest'

const mocks = vi.hoisted(() => ({
  fetchLoginLogs: vi.fn(),
  deleteLoginLog: vi.fn(),
  deleteLoginLogs: vi.fn(),
  confirm: vi.fn().mockResolvedValue(undefined)
}))

vi.mock('@/api/logs', () => ({
  fetchLoginLogs: mocks.fetchLoginLogs,
  deleteLoginLog: mocks.deleteLoginLog,
  deleteLoginLogs: mocks.deleteLoginLogs
}))

vi.mock('@/ui/feedback', () => ({
  ElMessage: {
    success: vi.fn(),
    error: vi.fn()
  },
  ElMessageBox: {
    confirm: mocks.confirm
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
    mocks.deleteLoginLog.mockResolvedValue({ code: 'OK' })
    mocks.deleteLoginLogs.mockResolvedValue({ code: 'OK' })
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

  it('deletes a login log with the id returned by the API', async () => {
    mocks.fetchLoginLogs.mockResolvedValue({
      list: [{ id: 7, username: 'admin', ip: '127.0.0.1', status: true, errorMessage: '', agent: '', createdAt: '2026-07-13' }],
      total: 1,
      page: 1,
      pageSize: 10
    })
    const wrapper = mount(LoginLogView, {
      global: {
        plugins: [UiComponents]
      }
    })

    await flushPromises()
    expect(wrapper.text()).toContain('7')
    const deleteButtons = wrapper.findAll('button').filter((button) => button.text() === 'Delete')
    await deleteButtons.at(-1)?.trigger('click')
    await flushPromises()

    expect(mocks.deleteLoginLog).toHaveBeenCalledWith(7)
  })

  it('batch deletes the ids returned by the API', async () => {
    mocks.fetchLoginLogs.mockResolvedValue({
      list: [{ id: 7, username: 'admin', ip: '127.0.0.1', status: true, errorMessage: '', agent: '', createdAt: '2026-07-13' }],
      total: 1,
      page: 1,
      pageSize: 10
    })
    const wrapper = mount(LoginLogView, {
      global: {
        plugins: [UiComponents]
      }
    })

    await flushPromises()
    const selectionCheckboxes = wrapper.findAll('input[type="checkbox"]')
    await selectionCheckboxes[1].setValue(true)
    const deleteButtons = wrapper.findAll('button').filter((button) => button.text() === 'Delete')
    await deleteButtons[0].trigger('click')
    await flushPromises()

    expect(mocks.deleteLoginLogs).toHaveBeenCalledWith([7])
  })
})
