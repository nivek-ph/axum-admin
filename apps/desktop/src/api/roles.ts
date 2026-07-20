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

export async function getRolePermissionIds(id: number) {
  const response = await http.get<never, ApiEnvelope<{ menuIds?: number[] }>>(`/roles/${id}/menus`, withAuthHeaders())
  return response.data?.menuIds ?? []
}
export function setRolePermissionIds(id: number, menuIds: number[]) {
  return http.put<never, ApiEnvelope>(`/roles/${id}/menus`, { menuIds: sortedIds(menuIds) }, withAuthHeaders())
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
