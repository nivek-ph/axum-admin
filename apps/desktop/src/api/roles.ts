import type { ApiEnvelope } from './core'
import { withAuthHeaders } from './core'
import { http } from './http'

export interface RoleResource {
  id: number
  code: string
  name: string
  status: string
  sort: number
  data_scope: string
  is_system: boolean
}

export interface RolePayload {
  code: string
  name: string
  status?: string
  sort?: number
  data_scope?: string
}

export async function listRoles() {
  const response = await http.get<never, ApiEnvelope<{ list?: RoleResource[] }>>('/roles', withAuthHeaders())
  return response.data?.list ?? []
}

export function createRole(payload: RolePayload) {
  return http.post<never, ApiEnvelope<{ role?: RoleResource }>>('/roles', payload, withAuthHeaders())
}
export function updateRole(id: number, payload: RolePayload) {
  return http.put<never, ApiEnvelope>(`/roles/${id}`, payload, withAuthHeaders())
}
export function deleteRole(id: number) {
  return http.delete<never, ApiEnvelope>(`/roles/${id}`, withAuthHeaders())
}

function sortedIds(ids: number[]) {
  return [...new Set(ids)].filter(Number.isFinite).sort((a, b) => a - b)
}

export interface RolePageAccess {
  menuIds: number[]
  effectiveMenuIds: number[]
  systemManaged: boolean
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
  systemManaged: boolean
}

export async function getRolePageAccess(id: number) {
  const response = await http.get<never, ApiEnvelope<RolePageAccess>>(`/roles/${id}/menus`, withAuthHeaders())
  return response.data ?? { menuIds: [], effectiveMenuIds: [], systemManaged: false }
}
export function setRolePageAccess(id: number, menuIds: number[]) {
  return http.put<never, ApiEnvelope>(`/roles/${id}/menus`, { menuIds: sortedIds(menuIds) }, withAuthHeaders())
}
export async function getRolePermissions(id: number) {
  const response = await http.get<never, ApiEnvelope<RolePermissions>>(`/roles/${id}/permissions`, withAuthHeaders())
  return response.data ?? { permissions: [], catalog: [], systemManaged: false }
}
export function setRolePermissions(id: number, permissions: string[]) {
  return http.put<never, ApiEnvelope>(
    `/roles/${id}/permissions`,
    { permissions: [...new Set(permissions)].sort() },
    withAuthHeaders(),
  )
}
export async function getRoleDeptIds(id: number) {
  const response = await http.get<never, ApiEnvelope<{ deptIds?: number[] }>>(`/roles/${id}/depts`, withAuthHeaders())
  return response.data?.deptIds ?? []
}
export function setRoleDeptIds(id: number, deptIds: number[]) {
  return http.put<never, ApiEnvelope>(`/roles/${id}/depts`, { deptIds: sortedIds(deptIds) }, withAuthHeaders())
}
export async function getRoleUserIds(id: number) {
  const response = await http.get<never, ApiEnvelope<number[]>>(`/roles/${id}/users`, withAuthHeaders())
  return response.data ?? []
}
export function setRoleUserIds(id: number, userIds: number[]) {
  return http.put<never, ApiEnvelope>(`/roles/${id}/users`, { userIds: sortedIds(userIds) }, withAuthHeaders())
}
