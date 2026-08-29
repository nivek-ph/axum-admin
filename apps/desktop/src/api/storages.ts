import type { ApiResponse } from './core'
import { withAuthHeaders } from './core'
import { http } from './http'

export type StorageDriver = 'local' | 's3'

export interface StorageRecord {
  id: number
  name: string
  code: string
  driver: StorageDriver
  root?: string | null
  bucket?: string | null
  region?: string | null
  endpoint?: string | null
  publicBaseUrl?: string | null
  virtualHostStyle: boolean
  hasAccessKey: boolean
  hasSecretKey: boolean
  enabled: boolean
  isDefault: boolean
  sort: number
  description: string
  createdAt: string
  updatedAt: string
}

export interface StoragePayload {
  name: string
  code: string
  driver: StorageDriver
  root?: string
  bucket?: string
  region?: string
  endpoint?: string
  publicBaseUrl?: string
  accessKey?: string
  secretKey?: string
  virtualHostStyle: boolean
  enabled: boolean
  sort: number
  description: string
}

export async function fetchStorages(filters: { keyword?: string; driver?: StorageDriver } = {}) {
  const response = await http.get<never, ApiResponse<{ list: StorageRecord[] }>>('/storages', {
    ...withAuthHeaders(),
    params: { keyword: filters.keyword || undefined, driver: filters.driver || undefined },
  })
  return response.data?.list ?? []
}

export async function createStorage(payload: StoragePayload) {
  const response = await http.post<never, ApiResponse<StorageRecord>>('/storages', payload, withAuthHeaders())
  return response.data!
}

export async function updateStorage(id: number, payload: StoragePayload) {
  const response = await http.put<never, ApiResponse<StorageRecord>>(`/storages/${id}`, payload, withAuthHeaders())
  return response.data!
}

export function setStorageStatus(id: number, enabled: boolean) {
  return http.patch<never, ApiResponse>(`/storages/${id}/status`, { enabled }, withAuthHeaders())
}

export function setDefaultStorage(id: number) {
  return http.put<never, ApiResponse>(`/storages/${id}/default`, undefined, withAuthHeaders())
}

export function deleteStorage(id: number) {
  return http.delete<never, ApiResponse>(`/storages/${id}`, withAuthHeaders())
}
