import { http } from './http';
import { withAuthHeaders, type ApiResponse } from './core';

export interface FileRecord {
  id: number;
  name: string;
  url: string;
  ext: string;
  tag: string;
  category: string;
  updatedAt: string;
}

export interface FileListResult {
  list: FileRecord[];
  total: number;
  page: number;
  pageSize: number;
}

export interface FileFilters {
  page?: number;
  pageSize?: number;
  keyword?: string;
  category?: string;
}

export function normalizeFileListResponse(payload: ApiResponse<FileListResult>): FileListResult {
  return {
    list: payload?.data?.list || [],
    total: payload?.data?.total || 0,
    page: payload?.data?.page || 1,
    pageSize: payload?.data?.pageSize || 10,
  };
}

export async function fetchFiles(filters: FileFilters = {}) {
  const response = await http.get('/files', {
    ...withAuthHeaders(),
    params: {
      page: filters.page || 1,
      pageSize: filters.pageSize || 10,
      keyword: filters.keyword || undefined,
      category: filters.category || undefined,
    }
  });
  return normalizeFileListResponse(response);
}

export async function importFileUrl(payload: { name: string; url: string; tag?: string; category?: string }) {
  return http.post('/files/import-url', payload, withAuthHeaders());
}

export async function renameFile(payload: { id: number; name: string }) {
  return http.patch(`/files/${payload.id}/name`, payload, withAuthHeaders());
}

export async function deleteFile(id: number) {
  return http.delete(`/files/${id}`, withAuthHeaders());
}

export async function uploadFile(
  file: File,
  metadata: { tag?: string; category?: string } = {},
  onProgress?: (progress: number) => void
) {
  const formData = new FormData();
  formData.append('file', file);

  const authorization = withAuthHeaders().headers.Authorization;
  const baseUrl = http.defaults.baseURL || '';
  const query = new URLSearchParams();
  if (metadata.tag) query.set('tag', metadata.tag);
  if (metadata.category) query.set('category', metadata.category);
  const url = `${baseUrl}/files/upload${query.size ? `?${query.toString()}` : ''}`;

  return new Promise<any>((resolve, reject) => {
    const xhr = new XMLHttpRequest();
    xhr.open('POST', url);
    xhr.responseType = 'json';
    xhr.setRequestHeader('Authorization', authorization);

    xhr.upload.onprogress = (event) => {
      if (!onProgress || !event.lengthComputable) return;
      onProgress(Math.round((event.loaded / event.total) * 100));
    };

    xhr.onload = () => {
      resolve(xhr.response);
    };

    xhr.onerror = () => {
      reject(new Error('upload failed'));
    };

    xhr.send(formData);
  });
}
