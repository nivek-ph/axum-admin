import { defineStore } from 'pinia';
import { computed, ref } from 'vue';

export interface CoreMenuItem {
  key: string;
  label: string;
  path: string;
}

const coreMenuItems: CoreMenuItem[] = [
  { key: 'dashboard', label: 'Dashboard', path: '/dashboard' },
  { key: 'users', label: 'Users', path: '/users' },
  { key: 'roles', label: 'Roles', path: '/roles' },
  { key: 'departments', label: 'Departments', path: '/departments' },
  { key: 'menus', label: 'Access catalog', path: '/menus' },
  { key: 'params', label: 'Params', path: '/params' },
  { key: 'dictionaries', label: 'Dictionaries', path: '/dictionaries' },
  { key: 'files', label: 'Files', path: '/files' },
  { key: 'audit-events', label: 'Audit events', path: '/audit-events' },
  { key: 'profile', label: 'Profile', path: '/profile' },
];

export interface RemoteMenuItem {
  name: string;
  path?: string;
  meta?: {
    title?: string;
  };
  children?: RemoteMenuItem[];
}

const coreMenuKeys = new Set(coreMenuItems.map((item) => item.key));
const coreMenuByKey = new Map(coreMenuItems.map((item) => [item.key, item]));
const unrestrictedRouteNames = new Set(['login', 'profile']);

function normalizePath(path: string) {
  return `/${path.replace(/^\/+/, '')}`;
}

function flattenRemoteMenus(remoteMenus: RemoteMenuItem[]): RemoteMenuItem[] {
  return remoteMenus.flatMap((item) => [item, ...flattenRemoteMenus(item.children || [])]);
}

export function buildCoreMenuItems(remoteMenus?: RemoteMenuItem[]) {
  if (!remoteMenus) {
    return coreMenuItems;
  }

  const remoteMap = new Map(
    flattenRemoteMenus(remoteMenus)
      .filter((item) => item.name && coreMenuKeys.has(item.name))
      .map((item) => {
        const coreItem = coreMenuByKey.get(item.name);
        return [
          item.name,
          {
            key: item.name,
            label: coreItem?.label || item.meta?.title || item.name,
            path: coreItem?.path || normalizePath(item.name),
          } satisfies CoreMenuItem,
        ];
      })
  );

  return coreMenuItems.filter((item) => remoteMap.has(item.key)).map((item) => remoteMap.get(item.key) || item);
}

export const useMenuStore = defineStore('menu', () => {
  const items = ref<CoreMenuItem[]>(coreMenuItems);
  const accessLoaded = ref(false);
  const allowedRouteNames = computed(() => new Set(items.value.map((item) => item.key)));

  function setItems(nextItems: CoreMenuItem[]) {
    items.value = nextItems;
  }

  function setAuthorizedMenus(remoteMenus: RemoteMenuItem[], allowAll = false) {
    items.value = allowAll ? coreMenuItems : buildCoreMenuItems(remoteMenus);
    accessLoaded.value = true;
  }

  function resetAccess() {
    items.value = coreMenuItems;
    accessLoaded.value = false;
  }

  function canAccessRouteName(routeName: string | symbol | null | undefined) {
    if (!routeName) return true;
    const name = String(routeName);
    if (unrestrictedRouteNames.has(name)) return true;
    if (!accessLoaded.value) return true;
    return allowedRouteNames.value.has(name);
  }

  function canAccessPath(path: string) {
    if (!accessLoaded.value) return true;
    const normalizedPath = normalizePath(path);
    if (normalizedPath === '/login' || normalizedPath === '/profile') return true;
    return items.value.some((item) => item.path === normalizedPath || normalizePath(item.key) === normalizedPath);
  }

  function firstAuthorizedRouteName() {
    return items.value[0]?.key || 'profile';
  }

  function firstAuthorizedPath() {
    return items.value[0]?.path || '/profile';
  }

  return {
    items,
    accessLoaded,
    setItems,
    setAuthorizedMenus,
    resetAccess,
    canAccessRouteName,
    canAccessPath,
    firstAuthorizedRouteName,
    firstAuthorizedPath,
  };
});
