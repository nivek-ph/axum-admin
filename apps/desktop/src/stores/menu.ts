import { create } from 'zustand'

export interface MenuItem {
  key: string
  label: string
  path?: string
  children: MenuItem[]
}

export interface RemoteMenuItem {
  name: string
  path?: string
  hidden?: boolean
  menuType?: string
  meta?: { title?: string }
  children?: RemoteMenuItem[]
}

export const coreMenuItems: MenuItem[] = [
  { key: 'dashboard', label: 'Dashboard', path: '/dashboard', children: [] },
  { key: 'users', label: 'Users', path: '/users', children: [] },
  { key: 'roles', label: 'Roles', path: '/roles', children: [] },
  { key: 'departments', label: 'Departments', path: '/departments', children: [] },
  { key: 'menus', label: 'Access catalog', path: '/menus', children: [] },
  { key: 'params', label: 'Params', path: '/params', children: [] },
  { key: 'dictionaries', label: 'Dictionaries', path: '/dictionaries', children: [] },
  { key: 'files', label: 'Files', path: '/files', children: [] },
  { key: 'sys-storage', label: 'Storages', path: '/sys-storage', children: [] },
  { key: 'audit-events', label: 'Audit events', path: '/audit-events', children: [] },
  { key: 'profile', label: 'Profile', path: '/profile', children: [] },
]

const coreMenuByKey = new Map(coreMenuItems.map((item) => [item.key, item]))

function buildMenuItem(remote: RemoteMenuItem): MenuItem | null {
  if (remote.hidden || remote.menuType === 'action') return null

  const children = (remote.children ?? []).map(buildMenuItem).filter((item): item is MenuItem => item !== null)
  if (remote.menuType === 'directory' || children.length > 0) {
    if (children.length === 0) return null
    return {
      key: remote.name,
      label: remote.meta?.title || remote.name,
      children,
    }
  }

  const page = coreMenuByKey.get(remote.name)
  if (!page) return null
  return {
    ...page,
    label: remote.meta?.title || page.label,
  }
}

export function buildMenuItems(remoteMenus: RemoteMenuItem[]) {
  return remoteMenus.map(buildMenuItem).filter((item): item is MenuItem => item !== null)
}

export function flattenMenuItems(items: MenuItem[]): MenuItem[] {
  return items.flatMap((item) => [item, ...flattenMenuItems(item.children)])
}

interface MenuState {
  items: MenuItem[]
  accessLoaded: boolean
  setAuthorizedMenus: (menus: RemoteMenuItem[]) => void
  resetAccess: () => void
  canAccess: (routeName: string) => boolean
  firstAuthorizedPath: () => string
}

export const useMenuStore = create<MenuState>((set, get) => ({
  items: [],
  accessLoaded: false,
  setAuthorizedMenus: (menus) => set({ items: buildMenuItems(menus), accessLoaded: true }),
  resetAccess: () => set({ items: [], accessLoaded: false }),
  canAccess: (routeName) => {
    if (routeName === 'profile' || routeName === 'login') return true
    if (!get().accessLoaded) return false
    return flattenMenuItems(get().items).some((item) => item.key === routeName)
  },
  firstAuthorizedPath: () => flattenMenuItems(get().items).find((item) => item.path)?.path ?? '/profile',
}))
