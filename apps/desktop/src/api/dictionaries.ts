import type { ApiEnvelope } from './core'
import { withAuthHeaders } from './core'
import { http } from './http'

export interface DictionaryRecord {
  id: number
  name: string
  type: string
  status?: boolean
  desc: string
  parentId?: number | null
}
export interface DictionaryDetailRecord {
  id: number
  label: string
  value: string
  extend: string
  status?: boolean
  sort: number
  sysDictionaryId: number
  parentId?: number | null
  level: number
  path: string
  children: DictionaryDetailRecord[]
}
export type DictionaryPayload = Omit<DictionaryRecord, 'id'> & { id?: number }
export type DictionaryDetailPayload = Pick<
  DictionaryDetailRecord,
  'label' | 'value' | 'extend' | 'sort' | 'sysDictionaryId' | 'parentId' | 'status'
> & { id?: number }

export async function fetchDictionaries(name = '') {
  const response = await http.get<never, ApiEnvelope<DictionaryRecord[]>>('/dictionaries', {
    ...withAuthHeaders(),
    params: { name: name || undefined },
  })
  return Array.isArray(response.data) ? response.data : []
}
export async function fetchDictionaryDetails(dictionaryId: number) {
  const response = await http.get<never, ApiEnvelope<{ list?: DictionaryDetailRecord[] }>>(
    `/dictionaries/${dictionaryId}/tree`,
    withAuthHeaders(),
  )
  return response.data?.list ?? []
}
export function createDictionary(payload: DictionaryPayload) {
  return http.post<never, ApiEnvelope>('/dictionaries', payload, withAuthHeaders())
}
export function updateDictionary(payload: DictionaryPayload & { id: number }) {
  return http.put<never, ApiEnvelope>(`/dictionaries/${payload.id}`, payload, withAuthHeaders())
}
export function deleteDictionary(id: number) {
  return http.delete<never, ApiEnvelope>(`/dictionaries/${id}`, withAuthHeaders())
}
export function createDictionaryDetail(payload: DictionaryDetailPayload) {
  return http.post<never, ApiEnvelope>(`/dictionaries/${payload.sysDictionaryId}/tree`, payload, withAuthHeaders())
}
export function updateDictionaryDetail(payload: DictionaryDetailPayload & { id: number }) {
  return http.put<never, ApiEnvelope>(
    `/dictionaries/${payload.sysDictionaryId}/tree/${payload.id}`,
    payload,
    withAuthHeaders(),
  )
}
export function deleteDictionaryDetail(dictionaryId: number, id: number) {
  return http.delete<never, ApiEnvelope>(`/dictionaries/${dictionaryId}/tree/${id}`, withAuthHeaders())
}
