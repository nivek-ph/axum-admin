import { flushPromises, mount } from '@vue/test-utils';
import { createPinia, setActivePinia } from 'pinia';
import { describe, expect, it } from 'vitest';
import { createMemoryHistory, createRouter } from 'vue-router';

import AppHeader from './AppHeader.vue';
import { useAuthStore } from '@/stores/auth';
import { useMenuStore } from '@/stores/menu';

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
  it('clears the session and returns to login when logging out', async () => {
    const pinia = createPinia();
    setActivePinia(pinia);
    const router = createTestRouter();
    await router.push('/dashboard');
    await router.isReady();

    const authStore = useAuthStore();
    const menuStore = useMenuStore();
    authStore.setSession('token-123', {
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

    expect(authStore.token).toBe('');
    expect(authStore.userInfo).toBeNull();
    expect(menuStore.accessLoaded).toBe(false);
    expect(router.currentRoute.value.name).toBe('login');
  });
});
