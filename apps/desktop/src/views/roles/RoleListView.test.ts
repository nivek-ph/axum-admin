import { mount } from '@vue/test-utils';
import { createPinia, setActivePinia } from 'pinia';
import { UiComponents } from '@/components/ui';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { useAuthStore } from '@/stores/auth';
import { setLocale } from '@/i18n';

const mocks = vi.hoisted(() => ({
  getRolePermissionIds: vi.fn().mockResolvedValue([1]),
  setRolePermissionIds: vi.fn().mockResolvedValue({ code: 'OK' }),
  setRoleDeptIds: vi.fn().mockResolvedValue({ code: 'OK' }),
  updateRole: vi.fn().mockResolvedValue({ code: 'OK' }),
}));

vi.mock('@/api/authorities', () => ({
  fetchAuthorities: vi.fn().mockResolvedValue([
    {
      authorityId: 2,
      authorityName: 'Developer',
      parentId: 0,
      defaultRouter: 'dashboard',
      children: [],
      dataAuthorityId: [],
    },
    {
      authorityId: 1,
      authorityName: 'Super Admin',
      code: 'super_admin',
      parentId: 0,
      defaultRouter: 'dashboard',
      children: [],
      dataAuthorityId: [],
    },
    {
      authorityId: 3,
      authorityName: 'System Owner',
      code: 'super_admin',
      isSystem: true,
      parentId: 0,
      defaultRouter: 'dashboard',
      children: [],
      dataAuthorityId: [],
    },
  ]),
  createAuthority: vi.fn(),
  updateAuthority: vi.fn(),
  deleteAuthority: vi.fn(),
  fetchAuthorityUsers: vi.fn().mockResolvedValue([2]),
  setRoleUsers: vi.fn(),
}));

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
      meta: { title: 'System' },
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
          meta: { title: 'Users' },
          parameters: [],
          menuBtn: [],
          menuType: 'page',
          children: [
            {
              ID: 20,
              parentId: 2,
              path: '',
              name: 'users:list',
              hidden: true,
              component: '',
              sort: 1,
              meta: { title: 'List' },
              parameters: [],
              menuBtn: [],
              menuType: 'action',
              permission: 'system:user:list',
              permissionId: 1,
              method: 'GET',
              apiPath: '/api/users',
              children: [],
            },
            {
              ID: 21,
              parentId: 2,
              path: '',
              name: 'users:create',
              hidden: true,
              component: '',
              sort: 2,
              meta: { title: 'Create' },
              parameters: [],
              menuBtn: [],
              menuType: 'action',
              permission: 'system:user:create',
              permissionId: 3,
              method: 'POST',
              apiPath: '/api/users',
              children: [],
            },
          ],
        },
      ],
    },
  ]),
}));

vi.mock('@/api/users', () => ({
  fetchUsers: vi.fn().mockResolvedValue({
    list: [
      { ID: 1, userName: 'admin', nickName: 'admin', phone: '', email: '', enable: 1 },
      { ID: 2, userName: 'nick', nickName: 'nick', phone: '', email: '', enable: 1 },
    ],
    total: 2,
    page: 1,
    pageSize: 10,
  }),
}));

vi.mock('@/api/system/roles', () => ({
  getRolePermissionIds: mocks.getRolePermissionIds,
  setRolePermissionIds: mocks.setRolePermissionIds,
  getRoleDeptIds: vi.fn().mockResolvedValue([1]),
  setRoleDeptIds: mocks.setRoleDeptIds,
  updateRole: mocks.updateRole,
}));

vi.mock('@/api/system/permissions', () => ({
  listPermissions: vi.fn().mockResolvedValue([
    {
      id: 1,
      module_key: 'system',
      resource: 'user',
      action: 'list',
      code: 'system:user:list',
      name: 'List Users',
      type: 'action',
      status: 'enabled',
    },
    {
      id: 2,
      module_key: 'system',
      resource: 'role',
      action: 'update',
      code: 'system:role:update',
      name: 'Update Roles',
      type: 'action',
      status: 'enabled',
    },
  ]),
}));

vi.mock('@/api/system/depts', () => ({
  listDepts: vi.fn().mockResolvedValue([
    {
      id: 1,
      parent_id: null,
      name: 'Head Office',
      code: 'head_office',
      sort: 0,
      status: 'enabled',
      children: [
        {
          id: 2,
          parent_id: 1,
          name: 'Product',
          code: 'product',
          sort: 1,
          status: 'enabled',
          children: [],
        },
      ],
    },
  ]),
}));

import RoleListView from './RoleListView.vue';

async function flushWorkbench() {
  await Promise.resolve();
  await Promise.resolve();
  await Promise.resolve();
  await Promise.resolve();
}

function mountWithAuthority(authorityId = 1, permissions?: string[]) {
  const pinia = createPinia();
  setActivePinia(pinia);
  const authStore = useAuthStore();
  authStore.setSession('token-123', {
    ID: authorityId === 1 ? 1 : 2,
    userName: authorityId === 1 ? 'admin' : 'nick',
    nickName: authorityId === 1 ? 'admin' : 'nick',
    authority: {
      authorityId,
      authorityName: authorityId === 1 ? 'Super Admin' : 'Developer',
      defaultRouter: 'dashboard',
    },
    permissions:
      permissions ??
      (authorityId === 1
        ? []
        : ['system:role:list', 'system:role:update-permission']),
  });

  return mount(RoleListView, {
    global: {
      plugins: [pinia, UiComponents],
    },
  });
}

describe('RoleListView', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    window.localStorage.clear();
    setLocale('en-US');
  });

  it('renders a role permission workbench', async () => {
    const wrapper = mountWithAuthority();

    await flushWorkbench();
    const header = wrapper.find('.role-workspace-header');
    expect(wrapper.text()).toContain('Roles');
    expect(wrapper.text()).toContain('New');
    expect(header.text()).not.toContain('Total 2 roles');
    expect(wrapper.text()).toContain('Function permissions');
    expect(wrapper.text()).toContain('Basic Info');
    expect(wrapper.text()).toContain('Menu Authorization');
    expect(wrapper.text()).toContain('Permission Authorization');
    expect(wrapper.text()).toContain('Data Scope');
    expect(wrapper.text()).toContain('Assigned Users');
    expect(wrapper.text()).toContain('Developer');
    expect(wrapper.text()).toContain('Super Admin');
    expect(wrapper.text()).toContain('Users');
  });

  it('translates role workbench chrome and backend menu labels to Chinese', async () => {
    setLocale('zh-CN');
    const wrapper = mountWithAuthority();

    await flushWorkbench();

    expect(wrapper.text()).toContain('功能权限');
    expect(wrapper.text()).toContain('基础信息');
    expect(wrapper.text()).toContain('菜单授权');
    expect(wrapper.text()).toContain('权限授权');
    expect(wrapper.text()).toContain('数据范围');
    expect(wrapper.text()).toContain('分配用户');
    expect(wrapper.text()).toContain('用户管理');
    expect(wrapper.text()).toContain('页面访问');
    expect(wrapper.text()).toContain('列表');
    expect(wrapper.text()).toContain('新增');
    expect(wrapper.text()).not.toContain('Function permissions');
    expect(wrapper.text()).not.toContain('Page access');
  });

  it('edits page and action permissions for the selected role', async () => {
    const wrapper = mountWithAuthority();

    await flushWorkbench();
    await wrapper.find('[data-test="role-list"] button:first-child').trigger('click');
    await flushWorkbench();

    const developerMenuCheckbox = wrapper.find('[data-test="menu-permission-2-2"]');
    expect((developerMenuCheckbox.element as HTMLInputElement).checked).toBe(true);
    await developerMenuCheckbox.setValue(false);

    await wrapper.find('[data-test="save-function-permissions"]').trigger('click');
    await flushWorkbench();

    expect(mocks.setRolePermissionIds).toHaveBeenCalledWith(2, []);
  });

  it('preserves non-menu permission resources when menu authorization changes', async () => {
    mocks.getRolePermissionIds.mockResolvedValueOnce([1, 2]);
    const wrapper = mountWithAuthority();

    await flushWorkbench();
    await wrapper.find('[data-test="role-list"] button:first-child').trigger('click');
    await flushWorkbench();

    await wrapper.find('[data-test="menu-permission-2-2"]').setValue(false);
    await wrapper.find('[data-test="save-function-permissions"]').trigger('click');
    await flushWorkbench();

    expect(mocks.setRolePermissionIds).toHaveBeenCalledWith(2, [2]);
  });

  it('edits unified permission resources for the selected role', async () => {
    const wrapper = mountWithAuthority();

    await flushWorkbench();
    await wrapper.find('[data-test="role-list"] button:first-child').trigger('click');
    await wrapper.find('[data-test="permission-authorization-tab"]').trigger('click');
    await flushWorkbench();

    expect(wrapper.text()).toContain('List Users');
    expect(wrapper.text()).toContain('system:user:list');

    await wrapper.find('[data-test="permission-resource-2"]').setValue(true);
    await wrapper.find('[data-test="save-role-permissions"]').trigger('click');
    await flushWorkbench();

    expect(mocks.setRolePermissionIds).toHaveBeenCalledWith(2, [1, 2]);
  });

  it('edits data scope and custom departments for the selected role', async () => {
    const wrapper = mountWithAuthority();

    await flushWorkbench();
    await wrapper.find('[data-test="role-list"] button:first-child').trigger('click');
    await wrapper.find('[data-test="data-scope-tab"]').trigger('click');
    await flushWorkbench();

    expect(wrapper.text()).toContain('Head Office');
    expect(wrapper.text()).toContain('Product');

    await wrapper.find('[data-test="data-scope-custom_depts"]').setValue(true);
    await wrapper.find('[data-test="role-dept-2"]').setValue(true);
    await wrapper.find('[data-test="save-data-scope"]').trigger('click');
    await flushWorkbench();

    expect(mocks.updateRole).toHaveBeenCalledWith(
      2,
      expect.objectContaining({ data_scope: 'custom_depts' })
    );
    expect(mocks.setRoleDeptIds).toHaveBeenCalledWith(2, [1, 2]);
  });

  it('disables permission editing for the super admin role', async () => {
    const wrapper = mountWithAuthority();

    await flushWorkbench();
    await wrapper.findAll('[data-test="role-list"] button')[1].trigger('click');
    await flushWorkbench();

    expect(wrapper.find('[data-test="save-function-permissions"]').exists()).toBe(false);

    const superAdminMenuCheckbox = wrapper.find('[data-test="menu-permission-2-1"]');
    expect((superAdminMenuCheckbox.element as HTMLInputElement).disabled).toBe(true);

    const superAdminActionCheckbox = wrapper.find('[data-test="action-permission-system:user:list-1"]');
    expect((superAdminActionCheckbox.element as HTMLInputElement).disabled).toBe(true);
  });

  it('disables permission editing for system roles that do not use the system role id', async () => {
    const wrapper = mountWithAuthority();

    await flushWorkbench();
    await wrapper.findAll('[data-test="role-list"] button')[2].trigger('click');
    await flushWorkbench();

    expect(wrapper.find('[data-test="save-function-permissions"]').exists()).toBe(false);

    const systemRoleMenuCheckbox = wrapper.find('[data-test="menu-permission-2-3"]');
    expect((systemRoleMenuCheckbox.element as HTMLInputElement).disabled).toBe(true);
  });

  it('hides permission save controls without role permission', async () => {
    const wrapper = mountWithAuthority(2, ['system:role:list']);

    await flushWorkbench();

    expect(wrapper.find('[data-test="save-function-permissions"]').exists()).toBe(false);
    const menuCheckbox = wrapper.find('[data-test="menu-permission-2-2"]');
    expect((menuCheckbox.element as HTMLInputElement).disabled).toBe(true);

    const actionCheckbox = wrapper.find('[data-test="action-permission-system:user:list-2"]');
    expect((actionCheckbox.element as HTMLInputElement).disabled).toBe(true);
  });
});
