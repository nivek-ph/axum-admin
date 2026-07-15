import { createRouter, createWebHashHistory, type RouteRecordRaw } from 'vue-router';

import AppLayout from '@/layouts/AppLayout.vue';
import { useAuthStore } from '@/stores/auth';
import { useMenuStore } from '@/stores/menu';

const LoginView = () => import('@/views/auth/LoginView.vue');
const DashboardView = () => import('@/views/dashboard/DashboardView.vue');
const UserListView = () => import('@/views/users/UserListView.vue');
const RoleListView = () => import('@/views/roles/RoleListView.vue');
const MenuListView = () => import('@/views/menus/MenuListView.vue');
const ParamListView = () => import('@/views/params/ParamListView.vue');
const DictionaryListView = () => import('@/views/dictionaries/DictionaryListView.vue');
const FileLibraryView = () => import('@/views/files/FileLibraryView.vue');
const LoginLogView = () => import('@/views/logs/LoginLogView.vue');
const OperationLogView = () => import('@/views/logs/OperationLogView.vue');
const ProfileView = () => import('@/views/profile/ProfileView.vue');
const DeptTreeView = () => import('@/views/system/depts/DeptTreeView.vue');

const routes: RouteRecordRaw[] = [
  { path: '/login', name: 'login', component: LoginView },
  {
    path: '/',
    component: AppLayout,
    children: [
      { path: '', redirect: '/dashboard' },
      { path: 'dashboard', name: 'dashboard', component: DashboardView },
      { path: 'users', name: 'users', component: UserListView },
      { path: 'roles', name: 'roles', component: RoleListView },
      { path: 'menus', name: 'menus', component: MenuListView },
      { path: 'params', name: 'params', component: ParamListView },
      { path: 'dictionaries', name: 'dictionaries', component: DictionaryListView },
      { path: 'files', name: 'files', component: FileLibraryView },
      { path: 'login-logs', name: 'login-logs', component: LoginLogView },
      { path: 'operation-logs', name: 'operation-logs', component: OperationLogView },
      { path: 'profile', name: 'profile', component: ProfileView },
      { path: 'departments', name: 'departments', component: DeptTreeView },
    ],
  },
];

export function createAppRouter() {
  const router = createRouter({
    history: createWebHashHistory(),
    routes,
  });

  router.beforeEach((to) => {
    const authStore = useAuthStore();
    const menuStore = useMenuStore();
    if (to.name !== 'login' && !authStore.isAuthenticated) {
      return { name: 'login' };
    }
    if (to.name === 'login' && authStore.isAuthenticated) {
      const homeRouteName = authStore.homeRouteName;
      return router.hasRoute(homeRouteName) && menuStore.canAccessRouteName(homeRouteName) ? { name: homeRouteName } : { name: menuStore.firstAuthorizedRouteName() };
    }
    if (!menuStore.canAccessRouteName(to.name)) {
      return { name: menuStore.firstAuthorizedRouteName() };
    }
    return true;
  });

  return router;
}
