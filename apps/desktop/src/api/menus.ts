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
  children: MenuRecord[]
}

export interface MenuRoleSelection {
  authorityIds: number[]
  defaultRouterAuthorityIds: number[]
}

export interface MenuRoleMatrixItem {
  menuId: number
  authorityIds: number[]
}

export interface AssignedMenuRecord {
  menuId?: number
  ID?: number
  parentId?: number
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
    authorityIds: payload?.data?.authorityIds || [],
    defaultRouterAuthorityIds: payload?.data?.defaultRouterAuthorityIds || []
  }
}

export function normalizeAuthorityMenuSelection(payload: ApiResponse<{ menus?: AssignedMenuRecord[] }>) {
  const menus = Array.isArray(payload?.data?.menus) ? payload.data.menus : []
  return menus
    .map((item) => item.menuId ?? item.ID)
    .filter((id): id is number => typeof id === 'number')
}

export function normalizeMenuRoleMatrixResponse(payload: ApiResponse<{ items?: MenuRoleMatrixItem[] }>) {
  const items = Array.isArray(payload?.data?.items) ? payload.data.items : []
  return items.reduce<Record<number, number[]>>((acc, item) => {
    acc[item.menuId] = item.authorityIds || []
    return acc
  }, {})
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

export async function setMenuRoles(menuId: number, authorityIds: number[]) {
  return http.put(`/menus/${menuId}/roles`, { menuId, authorityIds }, withAuthHeaders())
}

export async function fetchMenuRoleMatrix() {
  const response = await http.get('/menus/role-matrix', withAuthHeaders())
  return normalizeMenuRoleMatrixResponse(response)
}

export async function fetchAuthorityMenus(authorityId: number) {
  const response = await http.get('/menus/authority', {
    ...withAuthHeaders(),
    params: { authorityId }
  })
  return normalizeAuthorityMenuSelection(response)
}

export async function setAuthorityMenus(authorityId: number, menuIds: number[]) {
  return http.put('/menus/authority', { authorityId, menuIds }, withAuthHeaders())
}
