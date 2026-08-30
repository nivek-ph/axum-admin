import type { ApiResponse } from './core'
import { withAuthHeaders } from './core'
import { ApiHttpError, http } from './http'

export const MAX_UPLOAD_BYTES = 1024 * 1024 * 1024

interface UploadSessionData {
  id: string
  offset: number
  totalSize: number
  chunkSize: number
}

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
  const response = await http.get<never, ApiResponse<FileListResult>>('/files', {
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
  return http.post<never, ApiResponse>('/files/import-url', payload, withAuthHeaders())
}
export function renameFile(payload: { id: number; name: string }) {
  return http.patch<never, ApiResponse>(`/files/${payload.id}/name`, payload, withAuthHeaders())
}
export function deleteFile(id: number) {
  return http.delete<never, ApiResponse>(`/files/${id}`, withAuthHeaders())
}
export async function uploadFile(
  file: File,
  metadata: { tag?: string; category?: string } = {},
  onProgress?: (progress: number) => void,
) {
  const resumeKey = `file-upload:${file.name}:${file.size}:${file.lastModified}`
  let session: UploadSessionData | undefined
  const savedId = localStorage.getItem(resumeKey)
  if (savedId) {
    try {
      const response = await http.get<never, ApiResponse<UploadSessionData>>(
        `/files/uploads/${savedId}`,
        withAuthHeaders(),
      )
      if (response.data?.totalSize === file.size) session = response.data
    } catch (error) {
      if (error instanceof ApiHttpError && error.body?.code === 'UPLOAD_NOT_FOUND') {
        localStorage.removeItem(resumeKey)
      } else {
        throw error
      }
    }
  }
  if (!session) {
    const response = await http.post<never, ApiResponse<UploadSessionData>>(
      '/files/uploads',
      { name: file.name, size: file.size, ...metadata },
      withAuthHeaders(),
    )
    session = response.data
    if (session) localStorage.setItem(resumeKey, session.id)
  }
  if (!session) throw new Error('Upload session was not created')
  let current = session
  onProgress?.(Math.round((current.offset / file.size) * 100) || 0)
  while (current.offset < file.size) {
    const chunk = file.slice(current.offset, current.offset + current.chunkSize)
    const response = await http.patch<never, ApiResponse<UploadSessionData>>(`/files/uploads/${current.id}`, chunk, {
      ...withAuthHeaders(),
      headers: {
        ...withAuthHeaders().headers,
        'Content-Type': 'application/octet-stream',
        'Upload-Offset': current.offset,
      },
    })
    if (!response.data) throw new Error('Upload offset was not returned')
    current = response.data
    onProgress?.(Math.round((current.offset / file.size) * 100))
  }
  const response = await http.post<never, ApiResponse>(
    `/files/uploads/${current.id}/complete`,
    undefined,
    withAuthHeaders(),
  )
  localStorage.removeItem(resumeKey)
  return response
}
