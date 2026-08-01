import type { ApiResponse } from './core'
import { withAuthHeaders } from './core'
import { http } from './http'

export interface ParamRecord {
  id: number
  name: string
  key: string
  value: string
  desc: string
}
export interface ParamFilters {
  page?: number
  pageSize?: number
  name?: string
  key?: string
}
export interface ParamListResult {
  list: ParamRecord[]
  total: number
  page: number
  pageSize: number
}

export async function fetchParams(filters: ParamFilters = {}) {
  const page = filters.page ?? 1
  const pageSize = filters.pageSize ?? 10
  const response = await http.get<never, ApiResponse<ParamListResult>>('/params', {
    ...withAuthHeaders(),
    params: { page, pageSize, name: filters.name || undefined, key: filters.key || undefined },
  })
  return {
    list: response.data?.list ?? [],
    total: response.data?.total ?? 0,
    page: response.data?.page ?? page,
    pageSize: response.data?.pageSize ?? pageSize,
  }
}

export function createParam(payload: Omit<ParamRecord, 'id'>) {
  return http.post<never, ApiResponse>('/params', payload, withAuthHeaders())
}
export function updateParam(payload: ParamRecord) {
  return http.put<never, ApiResponse>(`/params/${payload.id}`, payload, withAuthHeaders())
}
export function deleteParam(id: number) {
  return http.delete<never, ApiResponse>(`/params/${id}`, withAuthHeaders())
}
