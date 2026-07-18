import { computed, ref } from 'vue';
import { defineStore } from 'pinia';

import { clearAuthSession, readAuthSession, writeAuthSession } from './authStorage';

export interface AuthUserInfo {
  id: number;
  userName: string;
  nickName: string;
  headerImg?: string;
  homeRoute?: string;
  deptId?: number | null;
  deptName?: string;
  roles?: Array<{
    id: number;
    code: string;
    name: string;
  }>;
  roleIds?: number[];
  permissions?: string[];
}

export const useAuthStore = defineStore('auth', () => {
  const persisted = readAuthSession();
  const accessToken = ref(persisted.accessToken);
  const refreshToken = ref(persisted.refreshToken);
  const userInfo = ref<AuthUserInfo | null>(persisted.userInfo);

  const isAuthenticated = computed(
    () => accessToken.value.length > 0 && refreshToken.value.length > 0
  );
  const permissionSet = computed(() => new Set(userInfo.value?.permissions || []));
  const roles = computed(() => userInfo.value?.roles || []);
  const roleLabel = computed(() => roles.value.map((role) => role.name).filter(Boolean).join(' / '));
  const permissions = computed(() => userInfo.value?.permissions || []);
  const homeRouteName = computed(() => userInfo.value?.homeRoute?.replace(/^\/+/, '') || 'dashboard');
  const isSuperAdmin = computed(
    () =>
      roles.value.some((role) => role.code === 'super_admin')
  );

  function persistSession() {
    writeAuthSession({
      accessToken: accessToken.value,
      refreshToken: refreshToken.value,
      userInfo: userInfo.value,
    });
  }

  function setTokenPair(nextAccessToken: string, nextRefreshToken: string) {
    accessToken.value = nextAccessToken;
    refreshToken.value = nextRefreshToken;
    persistSession();
  }

  function setSession(
    nextAccessToken: string,
    nextRefreshToken: string,
    nextUserInfo: AuthUserInfo
  ) {
    accessToken.value = nextAccessToken;
    refreshToken.value = nextRefreshToken;
    userInfo.value = nextUserInfo;
    persistSession();
  }

  function setPermissions(nextPermissions: string[]) {
    if (!userInfo.value) return;
    userInfo.value = { ...userInfo.value, permissions: nextPermissions };
    persistSession();
  }

  function clearSession() {
    accessToken.value = '';
    refreshToken.value = '';
    userInfo.value = null;
    clearAuthSession();
  }

  function can(permission: string) {
    if (isSuperAdmin.value) return true;
    return permissionSet.value.has(permission);
  }

  return {
    accessToken,
    refreshToken,
    userInfo,
    isAuthenticated,
    roles,
    roleLabel,
    permissions,
    homeRouteName,
    permissionSet,
    isSuperAdmin,
    can,
    setTokenPair,
    setSession,
    setPermissions,
    clearSession,
  };
});
