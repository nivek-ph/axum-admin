import type { ApiEnvelope } from './core'
import { withAuthHeaders } from './core'
import { http } from './http'

export interface MenuRecord {
  id: number
  parentId: number
  path: string
  name: string
  sort: number
  meta?: { title?: string }
  menuType?: string
  permission?: string | null
  method?: string
  apiPath?: string
  apiBindings?: Array<{ method: string; pathPattern: string }>
  children: MenuRecord[]
}

export async function fetchMenuTree() {
  const response = await http.get<never, ApiEnvelope<MenuRecord[] | { menus?: MenuRecord[] }>>(
    '/menus/tree',
    withAuthHeaders(),
  )
  if (Array.isArray(response.data)) return response.data
  return response.data?.menus ?? []
}
