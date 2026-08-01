import type { ApiResponse } from './core'
import { withAuthHeaders } from './core'
import { http } from './http'

export interface RoleResource {
  id: number
  code: string
  name: string
  status: string
  sort: number
}

export interface RolePayload {
  code: string
  name: string
  status?: string
  sort?: number
}

export async function listRoles() {
  const response = await http.get<never, ApiResponse<{ list?: RoleResource[] }>>('/roles', withAuthHeaders())
  return response.data?.list ?? []
}

export function createRole(payload: RolePayload) {
  return http.post<never, ApiResponse<{ role?: RoleResource }>>('/roles', payload, withAuthHeaders())
}
export function updateRole(id: number, payload: RolePayload) {
  return http.put<never, ApiResponse>(`/roles/${id}`, payload, withAuthHeaders())
}
export function deleteRole(id: number) {
  return http.delete<never, ApiResponse>(`/roles/${id}`, withAuthHeaders())
}

function sortedIds(ids: number[]) {
  return [...new Set(ids)].filter(Number.isFinite).sort((a, b) => a - b)
}

export interface RolePageAccess {
  menuIds: number[]
  effectiveMenuIds: number[]
  protected: boolean
}

export interface PermissionCatalogItem {
  permission: string
  title: string
  menuType: 'page' | 'action'
  status: 'enabled' | 'disabled'
  effectivelyEnabled: boolean
  owningPageId: number
  owningPageTitle: string
  pageVisible: boolean
}

export interface RolePermissions {
  permissions: string[]
  catalog: PermissionCatalogItem[]
  protected: boolean
}

export async function getRolePageAccess(id: number) {
  const response = await http.get<never, ApiResponse<RolePageAccess>>(`/roles/${id}/menus`, withAuthHeaders())
  return response.data ?? { menuIds: [], effectiveMenuIds: [], protected: false }
}
export function setRolePageAccess(id: number, menuIds: number[]) {
  return http.put<never, ApiResponse>(`/roles/${id}/menus`, { menuIds: sortedIds(menuIds) }, withAuthHeaders())
}
export async function getRolePermissions(id: number) {
  const response = await http.get<never, ApiResponse<RolePermissions>>(`/roles/${id}/permissions`, withAuthHeaders())
  return response.data ?? { permissions: [], catalog: [], protected: false }
}
export function setRolePermissions(id: number, permissions: string[]) {
  return http.put<never, ApiResponse>(
    `/roles/${id}/permissions`,
    { permissions: [...new Set(permissions)].sort() },
    withAuthHeaders(),
  )
}
