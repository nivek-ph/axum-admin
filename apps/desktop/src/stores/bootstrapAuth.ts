import { getMenu, getUserInfo } from '@/api/auth';

import { useAuthStore } from './auth';
import { useMenuStore } from './menu';

export async function bootstrapAuthSession() {
  const authStore = useAuthStore();
  const menuStore = useMenuStore();
  if (!authStore.isAuthenticated) {
    menuStore.resetAccess();
    return;
  }

  try {
    const [userInfoResponse, menuResponse] = await Promise.all([
      getUserInfo(authStore.token),
      getMenu(authStore.token)
    ]);
    if (userInfoResponse.code !== 'OK' || menuResponse.code !== 'OK') {
      authStore.clearToken();
      menuStore.resetAccess();
      return;
    }

    const userInfo = userInfoResponse.data?.userInfo;
    if (!userInfo) {
      authStore.clearToken();
      menuStore.resetAccess();
      return;
    }

    authStore.setSession(authStore.token, userInfo);
    authStore.setPermissions(menuResponse.data?.permissions || []);
    menuStore.setAuthorizedMenus(menuResponse.data?.menus || [], authStore.isSuperAdmin);
  } catch {
    authStore.clearToken();
    menuStore.resetAccess();
  }
}
