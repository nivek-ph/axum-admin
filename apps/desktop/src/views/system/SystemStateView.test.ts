import { mount } from '@vue/test-utils'
import { UiComponents } from '@/components/ui'
import { describe, expect, it, vi } from 'vitest'
import { createMemoryHistory, createRouter } from 'vue-router'
import { createPinia, setActivePinia } from 'pinia'
import { useMenuStore } from '@/stores/menu'

vi.mock('@/api/system', () => ({
  fetchServerInfo: vi.fn().mockResolvedValue({
    os: { goos: 'darwin', numCpu: 8, compiler: 'rustc', goVersion: '0.1.0', numGoroutine: 0 },
    cpu: { cores: 8, cpus: [12, 18, 9, 24] },
    ram: { totalMb: 8192, usedMb: 2048, usedPercent: 25 },
    disk: [{ mountPoint: '/', totalGb: 500, usedGb: 125, usedPercent: 25 }]
  }),
  fetchSystemConfig: vi.fn().mockResolvedValue({
    system: { env: 'public', addr: '127.0.0.1:3000', 'db-type': 'pgsql' },
    captcha: { openCaptcha: 1, openCaptchaTimeOut: 300 },
    local: { storePath: './uploads' }
  })
}))

import SystemStateView from './SystemStateView.vue'

describe('SystemStateView', () => {
  it('renders system state heading', async () => {
    setActivePinia(createPinia())
    const router = createRouter({
      history: createMemoryHistory(),
      routes: [{ path: '/', component: SystemStateView }]
    })

    const wrapper = mount(SystemStateView, {
      global: {
        plugins: [UiComponents, router]
      }
    })

    await Promise.resolve()
    expect(wrapper.text()).toContain('System status')
  })

  it('hides the config page shortcut when the current role cannot access it', async () => {
    setActivePinia(createPinia())
    const menuStore = useMenuStore()
    menuStore.setAuthorizedMenus([{ name: 'system-state', path: 'system-state', meta: { title: 'System status' } }])
    const router = createRouter({
      history: createMemoryHistory(),
      routes: [{ path: '/', component: SystemStateView }]
    })

    const wrapper = mount(SystemStateView, {
      global: {
        plugins: [UiComponents, router]
      }
    })

    await Promise.resolve()
    expect(wrapper.text()).not.toContain('View config')
  })
})
