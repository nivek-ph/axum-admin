import { create } from 'zustand'

import { isSuperAdmin, useAuthStore } from './auth'

export interface MenuItem {
  key: string
  label: string
  path: string
}

export interface RemoteMenuItem {
  name: string
  path?: string
  meta?: { title?: string }
  children?: RemoteMenuItem[]
}

export const coreMenuItems: MenuItem[] = [
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
]

function flatten(items: RemoteMenuItem[]): RemoteMenuItem[] {
  return items.flatMap((item) => [item, ...flatten(item.children ?? [])])
}

export function buildMenuItems(remoteMenus: RemoteMenuItem[]) {
  const allowed = new Set(flatten(remoteMenus).map((item) => item.name))
  return coreMenuItems.filter((item) => allowed.has(item.key))
}

interface MenuState {
  items: MenuItem[]
  accessLoaded: boolean
  setAuthorizedMenus: (menus: RemoteMenuItem[], allowAll?: boolean) => void
  resetAccess: () => void
  canAccess: (routeName: string) => boolean
  firstAuthorizedPath: () => string
}

export const useMenuStore = create<MenuState>((set, get) => ({
  items: coreMenuItems,
  accessLoaded: false,
  setAuthorizedMenus: (menus, allowAll = false) =>
    set({ items: allowAll ? coreMenuItems : buildMenuItems(menus), accessLoaded: true }),
  resetAccess: () => set({ items: coreMenuItems, accessLoaded: false }),
  canAccess: (routeName) => {
    if (routeName === 'profile' || routeName === 'login') return true
    if (!get().accessLoaded) return true
    if (isSuperAdmin(useAuthStore.getState().userInfo)) return true
    return get().items.some((item) => item.key === routeName)
  },
  firstAuthorizedPath: () => get().items[0]?.path ?? '/profile',
}))
