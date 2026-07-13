import { mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { createMemoryHistory, createRouter } from 'vue-router'
import { describe, expect, it } from 'vitest'

import { UiComponents } from '@/components/ui'
import { useAuthStore } from '@/stores/auth'
import { useMenuStore } from '@/stores/menu'
import ProfileView from './ProfileView.vue'

describe('ProfileView', () => {
  it('hides quick actions for routes missing from the current role menu', () => {
    setActivePinia(createPinia())
    const authStore = useAuthStore()
    const menuStore = useMenuStore()
    authStore.setSession('token-123', {
      id: 2,
      userName: 'nick',
      nickName: 'nick',
      homeRoute: 'dashboard',
      roles: [{ id: 999, code: 'developer', name: 'Developer' }]
    })
    menuStore.setAuthorizedMenus([{ name: 'profile', path: 'profile', meta: { title: 'Profile' } }])
    const router = createRouter({
      history: createMemoryHistory(),
      routes: [{ path: '/', component: ProfileView }]
    })

    const wrapper = mount(ProfileView, {
      global: {
        plugins: [UiComponents, router]
      }
    })

    expect(wrapper.text()).not.toContain('Back to dashboard')
    expect(wrapper.text()).not.toContain('View users')
    expect(wrapper.text()).toContain('Developer')
  })
})
