import type { ApiEnvelope } from './core'
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
export interface CreateUserForm {
  userName: string
  nickName: string
  password: string
  phone?: string
  email?: string
  enable: number
  roleIds: number[]
}

export async function fetchUsers(page = 1, pageSize = 10) {
  const response = await http.get<never, ApiEnvelope<UserListResult>>('/users', {
    ...withAuthHeaders(),
    params: { page, pageSize },
  })
  return {
    list: response.data?.list ?? [],
    total: response.data?.total ?? 0,
    page: response.data?.page ?? page,
    pageSize: response.data?.pageSize ?? pageSize,
  }
}

export function createUser(form: CreateUserForm) {
  return http.post<never, ApiEnvelope>(
    '/users',
    {
      username: form.userName.trim(),
      nickName: form.nickName.trim(),
      password: form.password,
      phone: form.phone?.trim() || undefined,
      email: form.email?.trim() || undefined,
      enable: form.enable,
      roleIds: form.roleIds,
    },
    withAuthHeaders(),
  )
}

export function assignUserRoles(id: number, roleIds: number[]) {
  return http.put<never, ApiEnvelope>(`/users/${id}/roles`, { roleIds }, withAuthHeaders())
}

export function deleteUser(id: number) {
  return http.delete<never, ApiEnvelope>(`/users/${id}`, withAuthHeaders())
}

export function resetUserPassword(id: number, password = '123456') {
  return http.post<never, ApiEnvelope>(`/users/${id}/password/reset`, { id, password }, withAuthHeaders())
}

export interface ChangeOwnPasswordPayload {
  password: string
  newPassword: string
}

export function changeOwnPassword(payload: ChangeOwnPasswordPayload) {
  return http.put<never, ApiEnvelope>('/users/me/password', payload, withAuthHeaders())
}

export interface UpdateOwnProfilePayload {
  nickName?: string
  phone?: string
  email?: string
}

export function updateOwnProfile(payload: UpdateOwnProfilePayload) {
  return http.put<never, ApiEnvelope>(
    '/users/me',
    {
      nickName: payload.nickName?.trim() || undefined,
      phone: payload.phone?.trim() || undefined,
      email: payload.email?.trim() || undefined,
    },
    withAuthHeaders(),
  )
}
