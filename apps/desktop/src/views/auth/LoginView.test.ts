import { flushPromises, mount } from '@vue/test-utils';
import { createPinia, setActivePinia } from 'pinia';
import { createMemoryHistory, createRouter } from 'vue-router';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { fetchCaptcha, getMenu, getUserInfo, login } from '@/api/auth';
import { UiComponents } from '@/components/ui';
import { ElMessage } from '@/ui/feedback';
import { useAuthStore } from '@/stores/auth';
import { useMenuStore } from '@/stores/menu';
import LoginView from './LoginView.vue';

vi.mock('@/api/auth', () => ({
  login: vi.fn(),
  fetchCaptcha: vi.fn(),
  getUserInfo: vi.fn(),
  getMenu: vi.fn(),
}));

vi.mock('@/ui/feedback', () => ({
  ElMessage: {
    error: vi.fn(),
  },
}));

describe('LoginView', () => {
  beforeEach(() => {
    vi.mocked(fetchCaptcha).mockResolvedValue({
      code: 'OK',
      message: 'ok',
      data: {
        captchaLength: 4,
        picPath: 'data:image/svg+xml;base64,test',
        captchaId: 'captcha-123',
        openCaptcha: true,
      },
    });
  });

  it('persists permissions while handling missing optional menu details', async () => {
    const pinia = createPinia();
    setActivePinia(pinia);
    const router = createRouter({
      history: createMemoryHistory(),
      routes: [
        { path: '/login', name: 'login', component: LoginView },
        { path: '/profile', name: 'profile', component: { template: '<div />' } },
      ],
    });
    vi.mocked(login).mockResolvedValue({
      code: 'OK',
      message: 'ok',
      data: {
        accessToken: 'token-123',
        refreshToken: 'refresh-token',
        user: {
          id: 1,
          userName: 'operator',
          nickName: 'Operator',
          roles: [{ id: 999, code: 'operator', name: 'Operator' }],
        },
      },
    });
    vi.mocked(getUserInfo).mockResolvedValue({
      code: 'OK',
      message: 'ok',
      data: {},
    } as Awaited<ReturnType<typeof getUserInfo>>);
    vi.mocked(getMenu).mockResolvedValue({
      code: 'OK',
      message: 'ok',
      data: {
        permissions: ['system:user:create'],
      },
    } as Awaited<ReturnType<typeof getMenu>>);
    await router.push('/login');
    await router.isReady();

    const wrapper = mount(LoginView, {
      global: {
        plugins: [UiComponents, pinia, router],
      },
    });

    await flushPromises();
    await wrapper.find('input').setValue('operator');
    const inputs = wrapper.findAll('input');
    await inputs[1].setValue('secret');
    await inputs[2].setValue('abcd');
    await wrapper.find('button.w-full').trigger('click');
    await flushPromises();

    const authStore = useAuthStore();
    const menuStore = useMenuStore();
    expect(authStore.isAuthenticated).toBe(true);
    expect(authStore.can('system:user:create')).toBe(true);
    expect(menuStore.firstAuthorizedPath()).toBe('/profile');
    expect(router.currentRoute.value.name).toBe('profile');
    expect(ElMessage.error).not.toHaveBeenCalled();
    expect(login).toHaveBeenCalledWith(
      expect.objectContaining({
        username: 'operator',
        password: 'secret',
        captcha: 'abcd',
        captchaId: 'captcha-123',
      })
    );
  });

  it('grants full menu access when login user has the super_admin role code', async () => {
    const pinia = createPinia();
    setActivePinia(pinia);
    const router = createRouter({
      history: createMemoryHistory(),
      routes: [
        { path: '/login', name: 'login', component: LoginView },
        { path: '/dashboard', name: 'dashboard', component: { template: '<div />' } },
        { path: '/users', name: 'users', component: { template: '<div />' } },
      ],
    });
    vi.mocked(login).mockResolvedValue({
      code: 'OK',
      message: 'ok',
      data: {
        accessToken: 'token-123',
        refreshToken: 'refresh-token',
        user: {
          id: 1,
          userName: 'admin',
          nickName: 'Admin',
          homeRoute: 'users',
          roles: [{ id: 1, code: 'super_admin', name: 'Super Admin' }],
        },
      },
    });
    vi.mocked(getUserInfo).mockResolvedValue({
      code: 'OK',
      message: 'ok',
      data: {
        userInfo: {
          id: 1,
          userName: 'admin',
          nickName: 'Admin',
          homeRoute: 'users',
          roles: [{ id: 1, code: 'super_admin', name: 'Super Admin' }],
        },
      },
    });
    vi.mocked(getMenu).mockResolvedValue({
      code: 'OK',
      message: 'ok',
      data: {
        menus: [{ name: 'dashboard', path: 'dashboard', meta: { title: 'Dashboard' } }],
      },
    });
    await router.push('/login');
    await router.isReady();

    const wrapper = mount(LoginView, {
      global: {
        plugins: [UiComponents, pinia, router],
      },
    });

    await flushPromises();
    const inputs = wrapper.findAll('input');
    await inputs[0].setValue('admin');
    await inputs[1].setValue('secret');
    await inputs[2].setValue('abcd');
    await wrapper.find('button.w-full').trigger('click');
    await flushPromises();

    const menuStore = useMenuStore();
    expect(menuStore.canAccessRouteName('roles')).toBe(true);
    expect(router.currentRoute.value.name).toBe('users');
  });
});
