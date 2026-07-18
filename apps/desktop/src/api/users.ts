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
  roles?: Array<{
    id: number
    code: string
    name: string
  }>
  roleIds?: number[]
  permissions?: string[]
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
  roleIds?: number[]
  deptId?: number
}

export interface CreateUserPayload {
  username: string
  nickName: string
  password: string
  phone?: string
  email?: string
  enable: number
  roleIds?: number[]
  deptId?: number
}

export interface AssignUserRolesPayload {
  roleIds: number[]
}

export interface ChangeOwnPasswordPayload {
  password: string
  newPassword: string
}

export function normalizeUserListResponse(payload: any): UserListResult {
  return {
    list: payload?.data?.list || [],
    total: payload?.data?.total || 0,
    page: payload?.data?.page || 1,
    pageSize: payload?.data?.pageSize || 10
  }
}

export function buildCreateUserPayload(form: CreateUserForm): CreateUserPayload {
  return {
    username: form.userName.trim(),
    nickName: form.nickName.trim(),
    password: form.password,
    phone: form.phone?.trim() || undefined,
    email: form.email?.trim() || undefined,
    enable: form.enable,
    roleIds: form.roleIds,
    deptId: form.deptId
  }
}

export async function fetchUsers(page = 1, pageSize = 10) {
  const res = await http.get('/users', {
    ...withAuthHeaders(),
    params: { page, pageSize }
  })
  return normalizeUserListResponse(res)
}

export async function createUser(form: CreateUserForm) {
  return http.post('/users', buildCreateUserPayload(form), withAuthHeaders())
}

export async function assignUserRoles(id: number, payload: AssignUserRolesPayload) {
  return http.put(`/users/${id}/roles`, payload, withAuthHeaders())
}

export async function deleteUser(id: number) {
  return http.delete(`/users/${id}`, withAuthHeaders())
}

export async function resetUserPassword(id: number, password: string) {
  return http.post(`/users/${id}/password/reset`, { id: id, password }, withAuthHeaders())
}

export async function changeOwnPassword(payload: ChangeOwnPasswordPayload) {
  return http.put('/users/me/password', payload, withAuthHeaders())
}
