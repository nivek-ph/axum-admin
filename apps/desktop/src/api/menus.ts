import { http } from './http'
import { withAuthHeaders, type ApiResponse } from './core'

export interface MenuMeta {
  activeName: string
  keepAlive: boolean
  defaultMenu: boolean
  title: string
  icon: string
  closeTab: boolean
  transitionType: string
}

export interface MenuParameter {
  ID: number
  sysBaseMenuID: number
  type: string
  key: string
  value: string
}

export interface MenuButton {
  ID: number
  name: string
  desc: string
}

export interface MenuRecord {
  ID: number
  parentId: number
  path: string
  name: string
  hidden: boolean
  component: string
  sort: number
  meta: MenuMeta
  parameters: MenuParameter[]
  menuBtn: MenuButton[]
  menuType?: 'directory' | 'page' | 'action' | string
  permission?: string | null
  permissionId?: number | null
  method?: string | null
  apiPath?: string | null
  children: MenuRecord[]
}

export interface MenuRoleSelection {
  roleIds: number[]
}

export function normalizeMenuListResponse(payload: ApiResponse<MenuRecord[]>) {
  if (Array.isArray(payload?.data)) {
    return payload.data
  }
  if (Array.isArray((payload?.data as any)?.menus)) {
    return (payload.data as any).menus
  }
  return []
}

export function normalizeMenuRoleSelection(payload: ApiResponse<MenuRoleSelection>) {
  return {
    roleIds: payload?.data?.roleIds || []
  }
}

export async function fetchMenuList() {
  const response = await http.get('/menus', withAuthHeaders())
  return normalizeMenuListResponse(response)
}

export async function createMenu(payload: MenuRecord) {
  return http.post('/menus', payload, withAuthHeaders())
}

export async function updateMenu(payload: MenuRecord) {
  return http.put(`/menus/${payload.ID}`, payload, withAuthHeaders())
}

export async function deleteMenu(id: number) {
  return http.delete(`/menus/${id}`, withAuthHeaders())
}

export async function fetchMenuRoles(menuId: number) {
  const response = await http.get(`/menus/${menuId}/roles`, withAuthHeaders())
  return normalizeMenuRoleSelection(response)
}

export async function setMenuRoles(menuId: number, roleIds: number[]) {
  return http.put(`/menus/${menuId}/roles`, { menuId, roleIds }, withAuthHeaders())
}
