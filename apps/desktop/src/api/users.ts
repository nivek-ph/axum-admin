import type { ApiResponse } from './core'
import { withAuthHeaders } from './core'
import { http } from './http'

export interface UserRecord {
  id: number
  userName: string
  nickName: string
  phone: string
  email: string
  enable: number
  deptId?: number
  deptName?: string
  roles?: Array<{ id: number; code: string; name: string }>
  roleIds?: number[]
}

export interface UserListResult {
  list: UserRecord[]
  total: number
  page: number
  pageSize: number
}
export interface UserFilters {
  page?: number
  pageSize?: number
  keyword?: string
}
export interface CreateUserForm {
  userName: string
  nickName: string
  password: string
  phone?: string
  email?: string
  enable: number
  roleIds?: number[]
  deptId?: number
}

export async function fetchUsers(filters: UserFilters = {}) {
  const page = filters.page ?? 1
  const pageSize = filters.pageSize ?? 10
  const response = await http.get<never, ApiResponse<UserListResult>>('/users', {
    ...withAuthHeaders(),
    params: { page, pageSize, keyword: filters.keyword || undefined },
  })
  return {
    list: response.data?.list ?? [],
    total: response.data?.total ?? 0,
    page: response.data?.page ?? page,
    pageSize: response.data?.pageSize ?? pageSize,
  }
}

export function createUser(form: CreateUserForm) {
  return http.post<never, ApiResponse>(
    '/users',
    {
      username: form.userName.trim(),
      nickName: form.nickName.trim(),
      password: form.password,
      phone: form.phone?.trim() || undefined,
      email: form.email?.trim() || undefined,
      enable: form.enable,
      roleIds: form.roleIds?.length ? form.roleIds : undefined,
      deptId: form.deptId,
    },
    withAuthHeaders(),
  )
}

export function assignUserRoles(id: number, roleIds: number[]) {
  return http.put<never, ApiResponse>(`/users/${id}/roles`, { roleIds }, withAuthHeaders())
}

export interface EffectiveRoleSource {
  id: number
  code: string
  name: string
}

export interface EffectivePermission {
  permission: string
  roles: EffectiveRoleSource[]
}

export interface UserAccess {
  assignedRoles: Array<EffectiveRoleSource & { status: string; sort: number }>
  effectivePermissions: EffectivePermission[]
}

export async function getUserAccess(id: number) {
  const response = await http.get<never, ApiResponse<UserAccess>>(`/users/${id}/access`, withAuthHeaders())
  return (
    response.data ?? {
      assignedRoles: [],
      effectivePermissions: [],
    }
  )
}

export function deleteUser(id: number) {
  return http.delete<never, ApiResponse>(`/users/${id}`, withAuthHeaders())
}

export function resetUserPassword(id: number, password = '123456') {
  return http.post<never, ApiResponse>(`/users/${id}/password/reset`, { id, password }, withAuthHeaders())
}

export interface ChangeOwnPasswordPayload {
  password: string
  newPassword: string
}

export function changeOwnPassword(payload: ChangeOwnPasswordPayload) {
  return http.put<never, ApiResponse>('/users/me/password', payload, withAuthHeaders())
}

export interface UpdateOwnProfilePayload {
  nickName?: string
  phone?: string
  email?: string
}

export function updateOwnProfile(payload: UpdateOwnProfilePayload) {
  return http.put<never, ApiResponse>(
    '/users/me',
    {
      nickName: payload.nickName?.trim() || undefined,
      phone: payload.phone?.trim() || undefined,
      email: payload.email?.trim() || undefined,
    },
    withAuthHeaders(),
  )
}
