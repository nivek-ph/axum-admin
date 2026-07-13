import { flushPromises, mount } from '@vue/test-utils';
import { UiComponents } from '@/components/ui';
import { describe, expect, it, vi } from 'vitest';
import { setLocale } from '@/i18n';

const mocks = vi.hoisted(() => ({
  fetchMenuList: vi.fn().mockResolvedValue([
    {
      id: 1,
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
          id: 11,
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
}));

vi.mock('@/api/menus', () => ({
  fetchMenuList: mocks.fetchMenuList,
  createMenu: vi.fn(),
  updateMenu: vi.fn(),
  deleteMenu: vi.fn(),
  fetchMenuRoles: vi.fn().mockResolvedValue({
    roleIds: [],
  }),
  setMenuRoles: vi.fn(),
}));

import MenuListView from './MenuListView.vue';

describe('MenuListView', () => {
  it('renders the read-only access catalog', async () => {
    setLocale('zh-CN');
    const wrapper = mount(MenuListView, {
      global: {
        plugins: [UiComponents],
      },
    });

    await flushPromises();
    expect(wrapper.find('.admin-page > .page-hero > .page-hero-main').exists()).toBe(true);
    expect(wrapper.find('.page-hero-title').text()).toBe('菜单与权限');
    expect(wrapper.find('.page-hero-subtitle').text()).toBe('菜单定义由数据库迁移统一管理，此处仅供查看。');
    expect(wrapper.text()).toContain('菜单与权限');
    expect(wrapper.text()).toContain('菜单定义由数据库迁移统一管理，此处仅供查看。');
    expect(wrapper.text()).not.toContain('Menus and permissions');
    expect(wrapper.text()).not.toContain('read-only');
    expect(wrapper.text()).toContain('Users');
    expect(wrapper.text()).not.toContain('Internal action');
    await wrapper.get('[data-test="tree-toggle-1"]').trigger('click');
    expect(wrapper.text()).toContain('Internal action');
    expect(wrapper.text()).not.toContain('New');
  });
});
