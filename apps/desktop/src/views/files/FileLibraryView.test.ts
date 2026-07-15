import { mount } from '@vue/test-utils'
import { UiComponents } from '@/components/ui'
import { describe, expect, it, vi } from 'vitest'

vi.mock('@/api/files', () => ({
  fetchFiles: vi.fn().mockResolvedValue({
    list: [],
    total: 0,
    page: 1,
    pageSize: 10
  }),
  importFileUrl: vi.fn(),
  uploadFile: vi.fn(),
  renameFile: vi.fn(),
  deleteFile: vi.fn()
}))

import FileLibraryView from './FileLibraryView.vue'

describe('FileLibraryView', () => {
  it('renders file library heading', async () => {
    const wrapper = mount(FileLibraryView, {
      global: {
        plugins: [UiComponents]
      }
    })

    await Promise.resolve()
    expect(wrapper.text()).toContain('File management')
  })
})
