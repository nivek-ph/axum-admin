import { flushPromises, mount } from '@vue/test-utils'
import { UiComponents } from '@/components/ui'
import { describe, expect, it, vi } from 'vitest'

const mocks = vi.hoisted(() => ({
  fetchMenuList: vi.fn().mockResolvedValue([
    {
      ID: 1,
      parentId: 0,
      path: 'users',
      name: 'users',
      hidden: false,
      component: 'view/users/index.vue',
      sort: 1,
      meta: { title: 'Users' },
      parameters: [],
      menuBtn: [],
      menuType: 'page',
      children: [
        {
          ID: 11,
          parentId: 1,
          path: '',
          name: 'users:internal-action',
          hidden: true,
          component: '',
          sort: 1,
          meta: { title: 'Internal action' },
          parameters: [],
          menuBtn: [],
          menuType: 'action',
          permission: 'system:user:list',
          permissionId: 1,
          method: 'GET',
          apiPath: '/api/users',
          children: [],
        },
      ],
    },
  ]),
}))

vi.mock('@/api/authorities', () => ({
  fetchAuthorities: vi.fn().mockResolvedValue([])
}))

vi.mock('@/api/menus', () => ({
  fetchMenuList: mocks.fetchMenuList,
  createMenu: vi.fn(),
  updateMenu: vi.fn(),
  deleteMenu: vi.fn(),
  fetchMenuRoles: vi.fn().mockResolvedValue({
    roleIds: []
  }),
  setMenuRoles: vi.fn()
}))

import MenuListView from './MenuListView.vue'

describe('MenuListView', () => {
  it('renders menu management actions', async () => {
    const wrapper = mount(MenuListView, {
      global: {
        plugins: [UiComponents]
      }
    })

    await flushPromises()
    expect(wrapper.text()).toContain('Menus')
    expect(wrapper.text()).toContain('New')
    expect(wrapper.text()).toContain('Users')
    expect(wrapper.text()).not.toContain('Internal action')
  })
})
