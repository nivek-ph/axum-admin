import { http } from './http'
import { withAuthHeaders, type ApiResponse } from './core'

export interface ApiRecord {
  ID: number
  path: string
  description: string
  apiGroup: string
  method: string
}

export interface ApiListResult {
  list: ApiRecord[]
  total: number
  page: number
  pageSize: number
}

export interface ApiSearchFilters {
  page?: number
  pageSize?: number
  path?: string
  description?: string
  apiGroup?: string
  method?: string
}

export interface ApiRoleSelection {
  authorityIds: number[]
}

export interface ApiRoleMatrixItem {
  path: string
  method: string
  authorityIds: number[]
}

export function normalizeApiListResponse(payload: ApiResponse<ApiListResult>) {
  return {
    list: payload?.data?.list || [],
    total: payload?.data?.total || 0,
    page: payload?.data?.page || 1,
    pageSize: payload?.data?.pageSize || 10
  }
}

export function normalizeApiRoleSelection(payload: ApiResponse<ApiRoleSelection>) {
  return {
    authorityIds: payload?.data?.authorityIds || []
  }
}

export function normalizeAuthorityApiListResponse(payload: ApiResponse<{ apis?: ApiRecord[] }>) {
  return Array.isArray(payload?.data?.apis) ? payload.data.apis : []
}

export function apiPermissionKey(path: string, method: string) {
  return `${method} ${path}`
}

export function normalizeApiRoleMatrixResponse(payload: ApiResponse<{ items?: ApiRoleMatrixItem[] }>) {
  const items = Array.isArray(payload?.data?.items) ? payload.data.items : []
  return items.reduce<Record<string, number[]>>((acc, item) => {
    acc[apiPermissionKey(item.path, item.method)] = item.authorityIds || []
    return acc
  }, {})
}

export async function fetchApis(filters: ApiSearchFilters = {}) {
  const response = await http.get('/routes', {
    ...withAuthHeaders(),
    params: {
      page: filters.page || 1,
      pageSize: filters.pageSize || 10,
      path: filters.path || undefined,
      description: filters.description || undefined,
      apiGroup: filters.apiGroup || undefined,
      method: filters.method || undefined
    }
  })
  return normalizeApiListResponse(response)
}

export async function fetchApiGroups() {
  const response = await http.get('/routes/groups', withAuthHeaders())
  return Array.isArray(response?.data?.groups) ? response.data.groups : []
}

export async function createApi(payload: ApiRecord) {
  return http.post('/routes', payload, withAuthHeaders())
}

export async function updateApi(payload: ApiRecord) {
  return http.put(`/routes/${payload.ID}`, payload, withAuthHeaders())
}

export async function deleteApi(id: number) {
  return http.delete(`/routes/${id}`, withAuthHeaders())
}

export async function fetchApiRoles(path: string, method: string) {
  const response = await http.get('/routes/roles', {
    ...withAuthHeaders(),
    params: { path, method }
  })
  return normalizeApiRoleSelection(response)
}

export async function setApiRoles(path: string, method: string, authorityIds: number[]) {
  return http.put('/routes/roles', { path, method, authorityIds }, withAuthHeaders())
}

export async function fetchApiRoleMatrix() {
  const response = await http.get('/routes/role-matrix', withAuthHeaders())
  return normalizeApiRoleMatrixResponse(response)
}

export async function fetchAuthorityApis(authorityId: number) {
  const response = await http.get('/routes/authority', {
    ...withAuthHeaders(),
    params: { authorityId }
  })
  return normalizeAuthorityApiListResponse(response)
}
