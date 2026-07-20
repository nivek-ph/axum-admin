import type { ApiEnvelope } from './core'
import { withAuthHeaders } from './core'
import { http } from './http'

export interface FileRecord {
  id: number
  name: string
  url: string
  ext: string
  tag: string
  category: string
  updatedAt: string
}
export interface FileFilters {
  page?: number
  pageSize?: number
  keyword?: string
  category?: string
}
export interface FileListResult {
  list: FileRecord[]
  total: number
  page: number
  pageSize: number
}

export async function fetchFiles(filters: FileFilters = {}) {
  const page = filters.page ?? 1
  const pageSize = filters.pageSize ?? 10
  const response = await http.get<never, ApiEnvelope<FileListResult>>('/files', {
    ...withAuthHeaders(),
    params: { page, pageSize, keyword: filters.keyword || undefined, category: filters.category || undefined },
  })
  return {
    list: response.data?.list ?? [],
    total: response.data?.total ?? 0,
    page: response.data?.page ?? page,
    pageSize: response.data?.pageSize ?? pageSize,
  }
}
export function importFileUrl(payload: { name: string; url: string; tag?: string; category?: string }) {
  return http.post<never, ApiEnvelope>('/files/import-url', payload, withAuthHeaders())
}
export function renameFile(payload: { id: number; name: string }) {
  return http.patch<never, ApiEnvelope>(`/files/${payload.id}/name`, payload, withAuthHeaders())
}
export function deleteFile(id: number) {
  return http.delete<never, ApiEnvelope>(`/files/${id}`, withAuthHeaders())
}
export function uploadFile(
  file: File,
  metadata: { tag?: string; category?: string } = {},
  onProgress?: (progress: number) => void,
) {
  const formData = new FormData()
  formData.append('file', file)
  return http.post<never, ApiEnvelope>('/files/upload', formData, {
    ...withAuthHeaders(),
    params: { tag: metadata.tag || undefined, category: metadata.category || undefined },
    onUploadProgress: (event) => {
      if (onProgress && event.total) onProgress(Math.round((event.loaded / event.total) * 100))
    },
  })
}
