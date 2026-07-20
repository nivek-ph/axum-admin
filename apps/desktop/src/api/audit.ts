import type { ApiEnvelope } from './core'
import { withAuthHeaders } from './core'
import { http } from './http'

export interface AuditEventRecord {
  id: number
  actorId?: number
  actorLabel: string
  action: string
  resourceType: string
  resourceId?: string
  result: 'succeeded' | 'denied' | 'failed'
  reasonCode?: string
  sourceIp: string
  userAgent: string
  changes: unknown[]
  createdAt: string
}
export interface AuditFilters {
  page?: number
  pageSize?: number
  actor?: string
  action?: string
  resourceType?: string
  resourceId?: string
  result?: string
  startedAt?: string
  endedAt?: string
}
export interface AuditListResult {
  list: AuditEventRecord[]
  total: number
  page: number
  pageSize: number
}
export interface AuditFinding {
  title: string
  explanation: string
  eventIds: number[]
}
export interface AuditAnalysis {
  summary: string
  riskLevel: 'low' | 'medium' | 'high'
  findings: AuditFinding[]
}

export async function fetchAuditEvents(filters: AuditFilters = {}) {
  const page = filters.page ?? 1
  const pageSize = filters.pageSize ?? 10
  const response = await http.get<never, ApiEnvelope<AuditListResult>>('/audit/events', {
    ...withAuthHeaders(),
    params: {
      page,
      pageSize,
      actor: filters.actor || undefined,
      action: filters.action || undefined,
      resourceType: filters.resourceType || undefined,
      resourceId: filters.resourceId || undefined,
      result: filters.result || undefined,
      startedAt: filters.startedAt || undefined,
      endedAt: filters.endedAt || undefined,
    },
  })
  return {
    list: response.data?.list ?? [],
    total: response.data?.total ?? 0,
    page: response.data?.page ?? page,
    pageSize: response.data?.pageSize ?? pageSize,
  }
}
export async function fetchAuditEvent(id: number) {
  const response = await http.get<never, ApiEnvelope<AuditEventRecord | null>>(`/audit/events/${id}`, withAuthHeaders())
  return response.data ?? null
}
export async function analyzeAuditEvents(filters: Omit<AuditFilters, 'page' | 'pageSize'> = {}) {
  const response = await http.post<never, ApiEnvelope<AuditAnalysis>>(
    '/audit/events/analyze',
    {
      actor: filters.actor || undefined,
      action: filters.action || undefined,
      resourceType: filters.resourceType || undefined,
      resourceId: filters.resourceId || undefined,
      result: filters.result || undefined,
      startedAt: filters.startedAt || undefined,
      endedAt: filters.endedAt || undefined,
    },
    { ...withAuthHeaders(), timeout: 65_000 },
  )
  return response.data
}
