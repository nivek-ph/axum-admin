import { http } from './http';
import { withAuthHeaders, type ApiResponse } from './core';

export interface DictionaryRecord {
  id: number;
  name: string;
  type: string;
  status?: boolean;
  desc: string;
  parentId?: number | null;
}

export interface DictionaryDetailRecord {
  id: number;
  label: string;
  value: string;
  extend: string;
  status?: boolean;
  sort: number;
  sysDictionaryId: number;
  parentId?: number | null;
  level: number;
  path: string;
  children: DictionaryDetailRecord[];
}

export interface DictionaryDetailPayload {
  id: number;
  label: string;
  value: string;
  extend: string;
  status?: boolean;
  sort: number;
  sysDictionaryId: number;
  parentId?: number | null;
}

export function normalizeDictionaryListResponse(payload: ApiResponse<DictionaryRecord[]>) {
  return Array.isArray(payload?.data) ? payload.data : [];
}

export function normalizeDictionaryDetailTreeResponse(payload: ApiResponse<{ list: DictionaryDetailRecord[] }>) {
  return payload?.data?.list || [];
}

export async function fetchDictionaries(name = '') {
  const response = await http.get('/dictionaries', {
    ...withAuthHeaders(),
    params: { name: name || undefined },
  });
  return normalizeDictionaryListResponse(response);
}

export async function fetchDictionaryDetails(sysDictionaryId: number) {
  const response = await http.get(`/dictionaries/${sysDictionaryId}/tree`, withAuthHeaders());
  return normalizeDictionaryDetailTreeResponse(response);
}

export async function createDictionary(payload: DictionaryRecord) {
  return http.post('/dictionaries', payload, withAuthHeaders());
}

export async function updateDictionary(payload: DictionaryRecord) {
  return http.put(`/dictionaries/${payload.id}`, payload, withAuthHeaders());
}

export async function deleteDictionary(id: number) {
  return http.delete(`/dictionaries/${id}`, withAuthHeaders());
}

export async function createDictionaryDetail(payload: DictionaryDetailPayload) {
  return http.post(
    `/dictionaries/${payload.sysDictionaryId}/tree`,
    {
      ...payload,
      level: 0,
      path: '',
      children: [],
    },
    withAuthHeaders()
  );
}

export async function updateDictionaryDetail(payload: DictionaryDetailPayload) {
  return http.put(
    `/dictionaries/${payload.sysDictionaryId}/tree/${payload.id}`,
    {
      ...payload,
      level: 0,
      path: '',
      children: [],
    },
    withAuthHeaders()
  );
}

export async function deleteDictionaryDetail(dictionaryId: number, id: number) {
  return http.delete(`/dictionaries/${dictionaryId}/tree/${id}`, withAuthHeaders());
}
