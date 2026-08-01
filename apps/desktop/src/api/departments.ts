import type { ApiResponse } from './core'
import { withAuthHeaders } from './core'
import { http } from './http'

export interface DeptRecord {
  id: number
  parent_id?: number | null
  name: string
  code: string
  sort: number
  status: string
  children?: DeptRecord[]
}

export interface DeptPayload {
  parent_id?: number | null
  name: string
  code: string
  sort?: number
  status?: string
}

export async function listDepartments() {
  const response = await http.get<never, ApiResponse<{ list?: DeptRecord[] }>>('/depts', withAuthHeaders())
  return response.data?.list ?? []
}
export function createDepartment(payload: DeptPayload) {
  return http.post<never, ApiResponse>('/depts', payload, withAuthHeaders())
}
export function updateDepartment(id: number, payload: DeptPayload) {
  return http.put<never, ApiResponse>(`/depts/${id}`, payload, withAuthHeaders())
}
export function deleteDepartment(id: number) {
  return http.delete<never, ApiResponse>(`/depts/${id}`, withAuthHeaders())
}
