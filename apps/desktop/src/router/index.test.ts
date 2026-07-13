import { createPinia, setActivePinia } from 'pinia';
import { describe, expect, it } from 'vitest';
import { createAppRouter } from './index';
import { useAuthStore } from '@/stores/auth';
import { useMenuStore } from '@/stores/menu';

describe('router', () => {
  it('includes core admin routes', () => {
    const router = createAppRouter();
    const routeNames = router.getRoutes().map((route) => route.name);

    expect(routeNames).toContain('login');
    expect(routeNames).toContain('dashboard');
    expect(routeNames).toContain('users');
    expect(routeNames).toContain('roles');
    expect(routeNames).toContain('departments');
    expect(routeNames).toContain('menus');
    expect(routeNames).not.toContain('permissions');
    expect(routeNames).not.toContain('api-permissions');
    expect(routeNames).not.toContain('apis');
    expect(routeNames).not.toContain('forbidden');
  });

  it('redirects unauthenticated users to login', async () => {
    setActivePinia(createPinia());
    const router = createAppRouter();
    const authStore = useAuthStore();
    authStore.clearToken();

    await router.push('/users');

    expect(router.currentRoute.value.name).toBe('login');
  });

  it('redirects authenticated users from login to their home route', async () => {
    setActivePinia(createPinia());
    const router = createAppRouter();
    const authStore = useAuthStore();
    const menuStore = useMenuStore();
    authStore.setSession('token-123', {
      id: 1,
      userName: 'operator',
      nickName: 'Operator',
      homeRoute: 'users',
      roles: [{ id: 999, code: 'operator', name: 'Operator' }],
    });
    menuStore.setAuthorizedMenus([{ name: 'users', path: 'users', meta: { title: 'Users' } }]);

    await router.push('/login');

    expect(router.currentRoute.value.name).toBe('users');
  });

  it('falls back when an authenticated user has an unknown home route', async () => {
    setActivePinia(createPinia());
    const router = createAppRouter();
    const authStore = useAuthStore();
    const menuStore = useMenuStore();
    authStore.setSession('token-123', {
      id: 1,
      userName: 'operator',
      nickName: 'Operator',
      homeRoute: 'missing',
      roles: [{ id: 999, code: 'operator', name: 'Operator' }],
    });
    menuStore.setAuthorizedMenus([{ name: 'users', path: 'users', meta: { title: 'Users' } }]);

    await router.push('/login');

    expect(router.currentRoute.value.name).toBe('users');
  });

  it('falls back when an authenticated user cannot access their home route', async () => {
    setActivePinia(createPinia());
    const router = createAppRouter();
    const authStore = useAuthStore();
    const menuStore = useMenuStore();
    authStore.setSession('token-123', {
      id: 1,
      userName: 'operator',
      nickName: 'Operator',
      homeRoute: 'roles',
      roles: [{ id: 999, code: 'operator', name: 'Operator' }],
    });
    menuStore.setAuthorizedMenus([{ name: 'users', path: 'users', meta: { title: 'Users' } }]);

    await router.push('/login');

    expect(router.currentRoute.value.name).toBe('users');
  });

  it('redirects authenticated users away from routes missing from their current menu', async () => {
    setActivePinia(createPinia());
    const router = createAppRouter();
    const authStore = useAuthStore();
    const menuStore = useMenuStore();
    authStore.setSession('token-123', {
      id: 1,
      userName: 'operator',
      nickName: 'Operator',
      roles: [{ id: 999, code: 'operator', name: 'Operator' }],
    });
    menuStore.setAuthorizedMenus([{ name: 'dashboard', path: 'dashboard', meta: { title: 'Dashboard' } }]);

    await router.push('/roles');

    expect(router.currentRoute.value.name).toBe('dashboard');
  });
});
