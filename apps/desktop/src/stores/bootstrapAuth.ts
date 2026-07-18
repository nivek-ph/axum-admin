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
      getUserInfo(authStore.accessToken),
      getMenu(authStore.accessToken)
    ]);
    if (userInfoResponse.code !== 'OK' || menuResponse.code !== 'OK') {
      authStore.clearSession();
      menuStore.resetAccess();
      return;
    }

    const userInfo = userInfoResponse.data?.userInfo;
    if (!userInfo) {
      authStore.clearSession();
      menuStore.resetAccess();
      return;
    }

    authStore.setSession(authStore.accessToken, authStore.refreshToken, userInfo);
    authStore.setPermissions(menuResponse.data?.permissions || []);
    menuStore.setAuthorizedMenus(menuResponse.data?.menus || [], authStore.isSuperAdmin);
  } catch {
    authStore.clearSession();
    menuStore.resetAccess();
  }
}
