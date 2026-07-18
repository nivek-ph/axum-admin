import { flushPromises, mount } from '@vue/test-utils';
import { createPinia, setActivePinia } from 'pinia';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { createMemoryHistory, createRouter } from 'vue-router';

import AppHeader from './AppHeader.vue';
import { useAuthStore } from '@/stores/auth';
import { useMenuStore } from '@/stores/menu';
import { ElMessage } from '@/ui/feedback';

const authApi = vi.hoisted(() => ({
  logout: vi.fn(),
}));

vi.mock('@/api/auth', () => ({
  logout: authApi.logout,
}));

function createTestRouter() {
  return createRouter({
    history: createMemoryHistory(),
    routes: [
      { path: '/dashboard', name: 'dashboard', component: { template: '<div />' } },
      { path: '/login', name: 'login', component: { template: '<div />' } },
    ],
  });
}

describe('AppHeader', () => {
  beforeEach(() => {
    authApi.logout.mockReset();
    authApi.logout.mockResolvedValue({ code: 'OK', message: 'signed out' });
  });

  it('clears the session and returns to login when logging out', async () => {
    const pinia = createPinia();
    setActivePinia(pinia);
    const router = createTestRouter();
    await router.push('/dashboard');
    await router.isReady();

    const authStore = useAuthStore();
    const menuStore = useMenuStore();
    authStore.setSession('token-123', 'refresh-token', {
      id: 1,
      userName: 'admin',
      nickName: 'admin',
      roles: [{ id: 1, code: 'super_admin', name: 'Super Admin' }],
    });
    menuStore.setAuthorizedMenus([{ name: 'dashboard', path: 'dashboard', meta: { title: 'Dashboard' } }]);

    const wrapper = mount(AppHeader, {
      global: {
        plugins: [pinia, router],
      },
    });

    expect(wrapper.find('.user-subtitle').text()).toBe('Super Admin');

    await wrapper.get('[data-test="logout-button"]').trigger('click');
    await flushPromises();

    expect(authApi.logout).toHaveBeenCalledOnce();
    expect(authStore.accessToken).toBe('');
    expect(authStore.userInfo).toBeNull();
    expect(menuStore.accessLoaded).toBe(false);
    expect(router.currentRoute.value.name).toBe('login');
  });

  it('clears the local session and warns when server logout fails', async () => {
    authApi.logout.mockRejectedValue(new Error('network unavailable'));
    const warning = vi.spyOn(ElMessage, 'warning');
    const pinia = createPinia();
    setActivePinia(pinia);
    const router = createTestRouter();
    await router.push('/dashboard');
    await router.isReady();

    const authStore = useAuthStore();
    const menuStore = useMenuStore();
    authStore.setSession('token-123', 'refresh-token', {
      id: 1,
      userName: 'admin',
      nickName: 'admin',
    });

    const wrapper = mount(AppHeader, {
      global: {
        plugins: [pinia, router],
      },
    });

    await wrapper.get('[data-test="logout-button"]').trigger('click');
    await flushPromises();

    expect(authApi.logout).toHaveBeenCalledOnce();
    expect(warning).toHaveBeenCalledWith('Server session may still be active');
    expect(authStore.accessToken).toBe('');
    expect(menuStore.accessLoaded).toBe(false);
    expect(router.currentRoute.value.name).toBe('login');
  });
});
