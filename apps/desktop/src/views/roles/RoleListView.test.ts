import { mount } from '@vue/test-utils';
import { createPinia, setActivePinia } from 'pinia';
import { UiComponents } from '@/components/ui';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { useAuthStore } from '@/stores/auth';
import { setLocale } from '@/i18n';

const mocks = vi.hoisted(() => ({
  listRoles: vi.fn(),
  createRole: vi.fn(),
  deleteRole: vi.fn(),
  getRoleUserIds: vi.fn().mockResolvedValue([2]),
  setRoleUserIds: vi.fn(),
  getRolePermissionIds: vi.fn().mockResolvedValue([1, 2, 20]),
  setRolePermissionIds: vi.fn().mockResolvedValue({ code: 'OK' }),
  setRoleDeptIds: vi.fn().mockResolvedValue({ code: 'OK' }),
  updateRole: vi.fn().mockResolvedValue({ code: 'OK' }),
}));

const roleFixtures = [
    {
      id: 2,
      code: 'developer',
      name: 'Developer',
      status: 'enabled',
      sort: 2,
      data_scope: 'all',
      is_system: false,
    },
    {
      id: 1,
      code: 'super_admin',
      name: 'Super Admin',
      status: 'enabled',
      sort: 1,
      data_scope: 'all',
      is_system: true,
    },
    {
      id: 3,
      code: 'super_admin',
      name: 'System Owner',
      status: 'enabled',
      sort: 3,
      data_scope: 'all',
      is_system: true,
    },
  ];

vi.mock('@/api/menus', () => ({
  fetchMenuList: vi.fn().mockResolvedValue([
    {
      id: 1,
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
          id: 2,
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
              id: 20,
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
              id: 21,
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
      { id: 1, userName: 'admin', nickName: 'admin', phone: '', email: '', enable: 1 },
      { id: 2, userName: 'nick', nickName: 'nick', phone: '', email: '', enable: 1 },
    ],
    total: 2,
    page: 1,
    pageSize: 10,
  }),
}));

vi.mock('@/api/system/roles', () => ({
  listRoles: mocks.listRoles,
  createRole: mocks.createRole,
  deleteRole: mocks.deleteRole,
  getRoleUserIds: mocks.getRoleUserIds,
  setRoleUserIds: mocks.setRoleUserIds,
  getRolePermissionIds: mocks.getRolePermissionIds,
  setRolePermissionIds: mocks.setRolePermissionIds,
  getRoleDeptIds: vi.fn().mockResolvedValue([1]),
  setRoleDeptIds: mocks.setRoleDeptIds,
  updateRole: mocks.updateRole,
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

function mountWithRole(roleId = 1, permissions?: string[]) {
  const pinia = createPinia();
  setActivePinia(pinia);
  const authStore = useAuthStore();
  authStore.setSession('token-123', {
    id: roleId === 1 ? 1 : 2,
    userName: roleId === 1 ? 'admin' : 'nick',
    nickName: roleId === 1 ? 'admin' : 'nick',
    roles: [{ id: roleId, code: roleId === 1 ? 'super_admin' : 'developer', name: roleId === 1 ? 'Super Admin' : 'Developer' }],
    permissions: permissions ?? (roleId === 1 ? [] : ['system:role:list', 'system:role:update-permission']),
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
    mocks.listRoles.mockResolvedValue(roleFixtures);
    window.localStorage.clear();
    setLocale('en-US');
  });

  it('renders a role permission workbench', async () => {
    const wrapper = mountWithRole();

    await flushWorkbench();
    const header = wrapper.find('.role-workspace-header');
    expect(wrapper.text()).toContain('Roles');
    expect(wrapper.text()).toContain('New');
    expect(header.text()).not.toContain('Total 2 roles');
    expect(wrapper.text()).toContain('Function permissions');
    expect(wrapper.text()).toContain('Basic Info');
    expect(wrapper.text()).toContain('Menu Authorization');
    expect(wrapper.text()).not.toContain('Permission Authorization');
    expect(wrapper.text()).toContain('Data Scope');
    expect(wrapper.text()).toContain('Assigned Users');
    expect(wrapper.text()).toContain('Developer');
    expect(wrapper.text()).toContain('Super Admin');
    expect(wrapper.text()).toContain('Users');
  });

  it('translates role workbench chrome and backend menu labels to Chinese', async () => {
    setLocale('zh-CN');
    const wrapper = mountWithRole();

    await flushWorkbench();

    expect(wrapper.text()).toContain('功能权限');
    expect(wrapper.text()).toContain('基础信息');
    expect(wrapper.text()).toContain('菜单授权');
    expect(wrapper.text()).not.toContain('权限授权');
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
    const wrapper = mountWithRole();

    await flushWorkbench();
    await wrapper.find('[data-test="role-list"] button:first-child').trigger('click');
    await flushWorkbench();

    const developerMenuCheckbox = wrapper.find('[data-test="menu-permission-2-2"]');
    expect((developerMenuCheckbox.element as HTMLInputElement).checked).toBe(true);
    await developerMenuCheckbox.setValue(false);

    await wrapper.find('[data-test="save-function-permissions"]').trigger('click');
    await flushWorkbench();

    expect(mocks.setRolePermissionIds).toHaveBeenCalledWith(2, [1]);
  });

  it('selects a newly created role using the backend-generated id', async () => {
    mocks.createRole.mockResolvedValueOnce({
      code: 'OK',
      data: { role: { id: 4, code: 'operator', name: 'Operator' } },
    });
    mocks.listRoles
      .mockResolvedValueOnce(roleFixtures)
      .mockResolvedValueOnce([
        ...roleFixtures,
        {
          id: 4,
          code: 'operator',
          name: 'Operator',
          status: 'enabled',
          sort: 0,
          data_scope: 'all',
          is_system: false,
        },
      ]);
    const wrapper = mountWithRole();

    await flushWorkbench();
    await wrapper.find('[data-test="open-create-role"]').trigger('click');
    expect(wrapper.text()).not.toContain('Role ID');
    await wrapper.find('[data-test="role-name-input"]').setValue('Operator');
    await wrapper.find('[data-test="role-code-input"]').setValue('operator');
    await wrapper.find('[data-test="role-dialog-save"]').trigger('click');
    await flushWorkbench();

    expect(mocks.createRole).toHaveBeenCalledWith({
      code: 'operator',
      name: 'Operator',
      status: 'enabled',
      sort: 0,
      data_scope: 'all',
    });
    const operator = wrapper.findAll('[data-test="role-list"] button')
      .find((button) => button.text().includes('Operator'));
    expect(operator?.classes()).toContain('is-active');
  });

  it('adds ancestors without selecting sibling actions', async () => {
    mocks.getRolePermissionIds.mockResolvedValueOnce([]);
    const wrapper = mountWithRole();

    await flushWorkbench();
    await wrapper.find('[data-test="role-list"] button:first-child').trigger('click');
    await flushWorkbench();

    const createAction = wrapper.find('[data-test="action-permission-system:user:create-2"]');
    const listAction = wrapper.find('[data-test="action-permission-system:user:list-2"]');
    await createAction.setValue(true);

    expect((wrapper.find('[data-test="menu-permission-2-2"]').element as HTMLInputElement).checked).toBe(true);
    expect((createAction.element as HTMLInputElement).checked).toBe(true);
    expect((listAction.element as HTMLInputElement).checked).toBe(false);

    await wrapper.find('[data-test="save-function-permissions"]').trigger('click');
    await flushWorkbench();
    expect(mocks.setRolePermissionIds).toHaveBeenCalledWith(2, [1, 2, 21]);
  });

  it('selects all descendants when page access is enabled directly', async () => {
    mocks.getRolePermissionIds.mockResolvedValueOnce([1]);
    const wrapper = mountWithRole();

    await flushWorkbench();
    await wrapper.find('[data-test="role-list"] button:first-child').trigger('click');
    await flushWorkbench();

    await wrapper.find('[data-test="menu-permission-2-2"]').setValue(true);

    expect((wrapper.find('[data-test="action-permission-system:user:list-2"]').element as HTMLInputElement).checked).toBe(true);
    expect((wrapper.find('[data-test="action-permission-system:user:create-2"]').element as HTMLInputElement).checked).toBe(true);

    await wrapper.find('[data-test="save-function-permissions"]').trigger('click');
    await flushWorkbench();
    expect(mocks.setRolePermissionIds).toHaveBeenCalledWith(2, [1, 2, 20, 21]);
  });

  it('sends the complete ancestor closure when menu authorization changes', async () => {
    mocks.getRolePermissionIds.mockResolvedValueOnce([1, 2, 20]);
    const wrapper = mountWithRole();

    await flushWorkbench();
    await wrapper.find('[data-test="role-list"] button:first-child').trigger('click');
    await flushWorkbench();

    await wrapper.find('[data-test="menu-permission-2-2"]').setValue(false);
    await wrapper.find('[data-test="save-function-permissions"]').trigger('click');
    await flushWorkbench();

    expect(mocks.setRolePermissionIds).toHaveBeenCalledWith(2, [1]);
  });

  it('edits data scope and custom departments for the selected role', async () => {
    const wrapper = mountWithRole();

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

    expect(mocks.updateRole).toHaveBeenCalledWith(2, expect.objectContaining({ data_scope: 'custom_depts' }));
    expect(mocks.setRoleDeptIds).toHaveBeenCalledWith(2, [1, 2]);
  });

  it('disables permission editing for the super admin role', async () => {
    const wrapper = mountWithRole();

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
    const wrapper = mountWithRole();

    await flushWorkbench();
    await wrapper.findAll('[data-test="role-list"] button')[2].trigger('click');
    await flushWorkbench();

    expect(wrapper.find('[data-test="save-function-permissions"]').exists()).toBe(false);

    const systemRoleMenuCheckbox = wrapper.find('[data-test="menu-permission-2-3"]');
    expect((systemRoleMenuCheckbox.element as HTMLInputElement).disabled).toBe(true);
  });

  it('hides permission save controls without role permission', async () => {
    const wrapper = mountWithRole(2, ['system:role:list']);

    await flushWorkbench();

    expect(wrapper.find('[data-test="save-function-permissions"]').exists()).toBe(false);
    const menuCheckbox = wrapper.find('[data-test="menu-permission-2-2"]');
    expect((menuCheckbox.element as HTMLInputElement).disabled).toBe(true);

    const actionCheckbox = wrapper.find('[data-test="action-permission-system:user:list-2"]');
    expect((actionCheckbox.element as HTMLInputElement).disabled).toBe(true);
  });
});
