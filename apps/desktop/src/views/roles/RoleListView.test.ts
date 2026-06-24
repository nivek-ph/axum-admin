import { mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { UiComponents } from '@/components/ui'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { useAuthStore } from '@/stores/auth'

const mocks = vi.hoisted(() => ({
  setApiRoles: vi.fn().mockResolvedValue({ code: 'OK' }),
  setMenuRoles: vi.fn().mockResolvedValue({ code: 'OK' })
}))

vi.mock('@/api/authorities', () => ({
  fetchAuthorities: vi.fn().mockResolvedValue([
    {
      authorityId: 1,
      authorityName: '开发',
      parentId: 0,
      defaultRouter: 'dashboard',
      children: [],
      dataAuthorityId: []
    },
    {
      authorityId: 888,
      authorityName: 'Super Admin',
      parentId: 0,
      defaultRouter: 'dashboard',
      children: [],
      dataAuthorityId: []
    }
  ]),
  createAuthority: vi.fn(),
  updateAuthority: vi.fn(),
  deleteAuthority: vi.fn(),
  fetchAuthorityUsers: vi.fn().mockResolvedValue([2]),
  setRoleUsers: vi.fn()
}))

vi.mock('@/api/menus', () => ({
  fetchMenuList: vi.fn().mockResolvedValue([
    {
      ID: 1,
      parentId: 0,
      path: 'system',
      name: 'system',
      hidden: false,
      component: 'view/system/index.vue',
      sort: 1,
      meta: { title: '系统管理' },
      parameters: [],
      menuBtn: [],
      children: [
        {
          ID: 2,
          parentId: 1,
          path: 'users',
          name: 'users',
          hidden: false,
          component: 'view/users/index.vue',
          sort: 2,
          meta: { title: '用户管理' },
          parameters: [],
          menuBtn: [],
          children: []
        }
      ]
    }
  ]),
  fetchAuthorityMenus: vi.fn().mockResolvedValue([1, 2]),
  fetchMenuRoleMatrix: vi.fn().mockResolvedValue({
    1: [1],
    2: [1]
  }),
  setAuthorityMenus: vi.fn(),
  setMenuRoles: mocks.setMenuRoles
}))

vi.mock('@/api/apis', () => ({
  apiPermissionKey: (path: string, method: string) => `${method} ${path}`,
  fetchAuthorityApis: vi.fn().mockResolvedValue([
    { ID: 1, path: '/api/users', method: 'GET', apiGroup: 'user', description: 'List users' }
  ]),
  fetchApis: vi.fn().mockResolvedValue({
    list: [{ ID: 1, path: '/api/users', method: 'GET', apiGroup: 'user', description: 'List users' }],
    total: 1,
    page: 1,
    pageSize: 500
  }),
  fetchApiRoleMatrix: vi.fn().mockResolvedValue({
    'GET /api/users': []
  }),
  setApiRoles: mocks.setApiRoles
}))

vi.mock('@/api/users', () => ({
  fetchUsers: vi.fn().mockResolvedValue({
    list: [
      { ID: 1, userName: 'admin', nickName: 'admin', phone: '', email: '', enable: 1 },
      { ID: 2, userName: 'nick', nickName: 'nick', phone: '', email: '', enable: 1 }
    ],
    total: 2,
    page: 1,
    pageSize: 10
  })
}))

import RoleListView from './RoleListView.vue'

async function flushWorkbench() {
  await Promise.resolve()
  await Promise.resolve()
  await Promise.resolve()
  await Promise.resolve()
}

function mountWithAuthority(authorityId = 888) {
  const pinia = createPinia()
  setActivePinia(pinia)
  const authStore = useAuthStore()
  authStore.setSession('token-123', {
    ID: authorityId === 888 ? 1 : 2,
    userName: authorityId === 888 ? 'admin' : 'nick',
    nickName: authorityId === 888 ? 'admin' : 'nick',
    authority: {
      authorityId,
      authorityName: authorityId === 888 ? 'Super Admin' : '开发',
      defaultRouter: 'dashboard'
    }
  })

  return mount(RoleListView, {
    global: {
      plugins: [pinia, UiComponents]
    }
  })
}

describe('RoleListView', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    window.localStorage.clear()
  })

  it('renders a role permission workbench', async () => {
    const wrapper = mountWithAuthority()

    await flushWorkbench()
    expect(wrapper.text()).toContain('Roles')
    expect(wrapper.text()).toContain('New role')
    expect(wrapper.text()).toContain('功能权限')
    expect(wrapper.text()).toContain('接口权限')
    expect(wrapper.text()).toContain('角色用户')
    expect(wrapper.text()).toContain('开发')
    expect(wrapper.text()).toContain('Super Admin')
    expect(wrapper.text()).toContain('用户管理')
  })

  it('edits menu and API permissions in a role matrix', async () => {
    const wrapper = mountWithAuthority()

    await flushWorkbench()
    const superAdminMenuCheckbox = wrapper.find('[data-test="menu-permission-2-888"]')
    expect((superAdminMenuCheckbox.element as HTMLInputElement).checked).toBe(false)
    await superAdminMenuCheckbox.setValue(true)
    await wrapper.find('[data-test="save-menu-permissions"]').trigger('click')
    await flushWorkbench()

    expect(mocks.setMenuRoles).toHaveBeenCalledWith(2, [1, 888])

    await wrapper.find('[data-test="api-permissions-tab"]').trigger('click')
    await flushWorkbench()

    expect(wrapper.text()).toContain('/api/users')
    const superAdminApiCheckbox = wrapper.find('[data-test="api-permission-GET-/api/users-888"]')
    expect((superAdminApiCheckbox.element as HTMLInputElement).checked).toBe(false)
    await superAdminApiCheckbox.setValue(true)
    await wrapper.find('[data-test="save-api-permissions"]').trigger('click')
    await flushWorkbench()

    expect(mocks.setApiRoles).toHaveBeenCalledWith('/api/users', 'GET', [888])

    await wrapper.find('[data-test="role-users-tab"]').trigger('click')
    await flushWorkbench()

    expect(wrapper.text()).toContain('Selected members')
    expect(wrapper.text()).toContain('1 / 2')
    expect(wrapper.text()).toContain('admin')
    expect(wrapper.text()).toContain('nick')
  })

  it('hides permission save controls without API permission', async () => {
    const wrapper = mountWithAuthority(1)

    await flushWorkbench()

    expect(wrapper.find('[data-test="save-menu-permissions"]').exists()).toBe(false)
    const menuCheckbox = wrapper.find('[data-test="menu-permission-2-888"]')
    expect((menuCheckbox.element as HTMLInputElement).disabled).toBe(true)

    await wrapper.find('[data-test="api-permissions-tab"]').trigger('click')
    await flushWorkbench()

    expect(wrapper.find('[data-test="save-api-permissions"]').exists()).toBe(false)
    const apiCheckbox = wrapper.find('[data-test="api-permission-GET-/api/users-888"]')
    expect((apiCheckbox.element as HTMLInputElement).disabled).toBe(true)
  })
})
