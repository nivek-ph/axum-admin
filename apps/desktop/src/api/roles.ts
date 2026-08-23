import type { ApiResponse } from './core'
import { withAuthHeaders } from './core'
import { http } from './http'
import type { MenuRecord } from './menus'

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

export interface RoleAccess {
  permissions: string[]
  tree: MenuRecord[]
  protected: boolean
}

export async function getRoleAccess(id: number) {
  const response = await http.get<never, ApiResponse<RoleAccess>>(`/roles/${id}/access`, withAuthHeaders())
  return response.data ?? { permissions: [], tree: [], protected: false }
}
export function setRoleAccess(id: number, permissions: string[]) {
  return http.put<never, ApiResponse>(
    `/roles/${id}/access`,
    { permissions: [...new Set(permissions)].sort() },
    withAuthHeaders(),
  )
}
