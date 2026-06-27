import { flushPromises, mount } from '@vue/test-utils';
import { createPinia, setActivePinia } from 'pinia';
import { UiComponents } from '@/components/ui';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { useAuthStore } from '@/stores/auth';

vi.mock('@/api/apis', () => ({
  fetchApiGroups: vi.fn().mockResolvedValue([]),
  fetchApis: vi.fn().mockResolvedValue({
    list: [
      {
        ID: 1,
        path: '/api/system/reload',
        description: 'Reload system',
        apiGroup: 'system',
        method: 'POST',
      },
    ],
    total: 0,
    page: 1,
    pageSize: 10,
  }),
  createApi: vi.fn(),
  updateApi: vi.fn(),
  deleteApi: vi.fn(),
  fetchApiRoles: vi.fn().mockResolvedValue({
    roleIds: [],
  }),
  setApiRoles: vi.fn(),
}));

vi.mock('@/api/authorities', () => ({
  fetchAuthorities: vi.fn().mockResolvedValue([]),
}));

import ApiListView from './ApiListView.vue';

function mountWithAuthority(authorityId = 1, permissions: string[] = []) {
  const pinia = createPinia();
  setActivePinia(pinia);
  const authStore = useAuthStore();
  authStore.setSession('token-123', {
    ID: authorityId === 1 ? 1 : 2,
    userName: authorityId === 1 ? 'admin' : 'dev',
    nickName: authorityId === 1 ? 'admin' : 'dev',
    authority: {
      authorityId,
      authorityName: authorityId === 1 ? 'Super Admin' : 'Developer',
      defaultRouter: 'dashboard',
    },
    permissions,
  });

  return mount(ApiListView, {
    global: {
      plugins: [pinia, UiComponents],
    },
  });
}

describe('ApiListView', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    window.localStorage.clear();
  });

  it('renders api management actions', async () => {
    const wrapper = mountWithAuthority();

    await flushPromises();
    expect(wrapper.text()).toContain('API directory');
    expect(wrapper.text()).toContain('New');
    expect(wrapper.text()).toContain('Assign roles');
    expect(wrapper.find('.inline-filter').exists()).toBe(true);
    expect(wrapper.findAll('.inline-filter .ui-select')).toHaveLength(2);
  });

  it('hides privileged api actions for list-only roles', async () => {
    const wrapper = mountWithAuthority(1001, ['system:api:list']);

    await flushPromises();
    expect(wrapper.text()).toContain('API directory');
    expect(wrapper.text()).not.toContain('New');
    expect(wrapper.text()).not.toContain('Assign roles');
    expect(wrapper.text()).not.toContain('Edit');
    expect(wrapper.text()).not.toContain('Delete');
  });
});
