import { flushPromises, mount } from '@vue/test-utils';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { QueryClient, VueQueryPlugin } from '@tanstack/vue-query';
import { UiComponents } from '@/components/ui';
import { createRouter, createMemoryHistory } from 'vue-router';
import { createPinia, setActivePinia } from 'pinia';

import UserListView from './UserListView.vue';
import { useAuthStore } from '@/stores/auth';

const mocks = vi.hoisted(() => ({
  fetchUsers: vi.fn().mockResolvedValue({
    list: [],
    total: 0,
    page: 1,
    pageSize: 10,
  }),
  createUser: vi.fn(),
  deleteUser: vi.fn(),
  resetUserPassword: vi.fn(),
  assignUserRoles: vi.fn(),
  listRoles: vi.fn().mockResolvedValue([
    {
      id: 1,
      code: 'super_admin',
      name: 'Super Admin',
      status: 'enabled',
      sort: 0,
      data_scope: 'all',
      is_system: true,
    },
    {
      id: 1001,
      code: 'dev',
      name: 'Dev',
      status: 'enabled',
      sort: 1,
      data_scope: 'all',
      is_system: false,
    },
  ]),
}));

vi.mock('@/api/users', () => ({
  fetchUsers: mocks.fetchUsers,
  createUser: mocks.createUser,
  deleteUser: mocks.deleteUser,
  resetUserPassword: mocks.resetUserPassword,
  assignUserRoles: mocks.assignUserRoles,
}));

vi.mock('@/api/system/roles', () => ({
  listRoles: mocks.listRoles,
}));

function mountView(permissions = ['system:user:create', 'system:user:assign-roles', 'system:user:reset-password', 'system:user:delete']) {
  const router = createRouter({
    history: createMemoryHistory(),
    routes: [{ path: '/', component: UserListView }],
  });
  const pinia = createPinia();
  setActivePinia(pinia);
  const authStore = useAuthStore();
  authStore.setSession('test-token', 'refresh-token', {
    id: 1,
    userName: 'tester',
    nickName: 'tester',
    permissions,
    roles: [],
  });
  const queryClient = new QueryClient({
    defaultOptions: {
      queries: {
        retry: false,
      },
    },
  });
  return mount(UserListView, {
    global: {
      plugins: [UiComponents, router, pinia, [VueQueryPlugin, { queryClient }]],
    },
  });
}

describe('UserListView', () => {
  beforeEach(() => {
    localStorage.clear();
    mocks.fetchUsers.mockResolvedValue({
      list: [],
      total: 0,
      page: 1,
      pageSize: 10,
    });
  });

  it('renders user management heading', () => {
    const wrapper = mountView();

    expect(wrapper.text()).toContain('Users');
  });

  it('opens a new-user dialog from the user page', async () => {
    const wrapper = mountView();

    await wrapper.find('[data-test="new-user-button"]').trigger('click');

    expect(wrapper.text()).toContain('New user');
    expect(wrapper.text()).toContain('Username');
    expect(wrapper.text()).toContain('Password');
    expect(wrapper.text()).toContain('Role');
    expect(wrapper.text()).toContain('Super Admin');
    expect(wrapper.find('[data-test="user-role-select"]').exists()).toBe(true);
    expect(wrapper.find('[data-test="user-role-select"] select').exists()).toBe(false);
  });

  it('opens a change-role dialog from an existing user row', async () => {
    mocks.fetchUsers.mockResolvedValue({
      list: [
        {
          id: 4,
          userName: 'nick',
          nickName: 'nick',
          phone: '',
          email: '',
          enable: 1,
          deptId: 1,
          deptName: 'Head Office',
          roleIds: [1001],
          roles: [{ id: 1001, code: 'dev', name: 'Dev' }],
        },
      ],
      total: 1,
      page: 1,
      pageSize: 10,
    });
    const wrapper = mountView();
    await flushPromises();

    await wrapper.find('[data-test="change-user-role-button"]').trigger('click');

    expect(wrapper.text()).toContain('Change user role');
    expect(wrapper.text()).toContain('nick');
    expect(wrapper.text()).toContain('Dev');
    expect(wrapper.find('[data-test="edit-user-role-select"]').exists()).toBe(true);
  });

  it('hides user action buttons without matching permissions', async () => {
    mocks.fetchUsers.mockResolvedValue({
      list: [
        {
          id: 4,
          userName: 'nick',
          nickName: 'nick',
          phone: '',
          email: '',
          enable: 1,
          deptId: 1,
          deptName: 'Head Office',
          roleIds: [1001],
          roles: [{ id: 1001, code: 'dev', name: 'Dev' }],
        },
      ],
      total: 1,
      page: 1,
      pageSize: 10,
    });
    const wrapper = mountView(['system:user:list']);
    await flushPromises();

    expect(wrapper.find('[data-test="new-user-button"]').exists()).toBe(false);
    expect(wrapper.find('[data-test="change-user-role-button"]').exists()).toBe(false);
    expect(wrapper.find('[data-test="reset-user-password-button"]').exists()).toBe(false);
    expect(wrapper.find('[data-test="delete-user-button"]').exists()).toBe(false);
    expect(wrapper.find('[data-test="user-row-no-actions"]').exists()).toBe(true);
  });
});
