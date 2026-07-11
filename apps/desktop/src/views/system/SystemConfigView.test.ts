import { mount } from '@vue/test-utils'
import { UiComponents } from '@/components/ui'
import { describe, expect, it, vi } from 'vitest'
import { createMemoryHistory, createRouter } from 'vue-router'
import { createPinia, setActivePinia } from 'pinia'
import { useMenuStore } from '@/stores/menu'

vi.mock('@/api/system', () => ({
  fetchServerInfo: vi.fn(),
  fetchSystemConfig: vi.fn().mockResolvedValue({
    system: { env: 'public', addr: '127.0.0.1:3000', 'db-type': 'pgsql' },
    captcha: { openCaptcha: 1, openCaptchaTimeOut: 300 },
    local: { storePath: './uploads' }
  })
}))

import SystemConfigView from './SystemConfigView.vue'

describe('SystemConfigView', () => {
  it('renders system config heading', async () => {
    setActivePinia(createPinia())
    const router = createRouter({
      history: createMemoryHistory(),
      routes: [{ path: '/', component: SystemConfigView }]
    })

    const wrapper = mount(SystemConfigView, {
      global: {
        plugins: [UiComponents, router]
      }
    })

    await Promise.resolve()
    expect(wrapper.text()).toContain('System config')
  })

  it('hides the status page shortcut when the current role cannot access it', async () => {
    setActivePinia(createPinia())
    const menuStore = useMenuStore()
    menuStore.setAuthorizedMenus([{ name: 'system-config', path: 'system-config', meta: { title: 'System config' } }])
    const router = createRouter({
      history: createMemoryHistory(),
      routes: [{ path: '/', component: SystemConfigView }]
    })

    const wrapper = mount(SystemConfigView, {
      global: {
        plugins: [UiComponents, router]
      }
    })

    await Promise.resolve()
    expect(wrapper.text()).not.toContain('View status')
  })
})
