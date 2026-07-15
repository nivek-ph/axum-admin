import { http } from './http';
import { withAuthHeaders, type ApiResponse } from './core';

export interface AuditEventRecord {
  id: number;
  actorId?: number;
  actorLabel: string;
  action: string;
  resourceType: string;
  resourceId?: string;
  result: 'succeeded' | 'denied' | 'failed';
  reasonCode?: string;
  sourceIp: string;
  userAgent: string;
  changes: unknown[];
  createdAt: string;
}

export interface PaginatedResult<T> {
  list: T[];
  total: number;
  page: number;
  pageSize: number;
}

export interface AuditEventFilters {
  page?: number;
  pageSize?: number;
  actor?: string;
  action?: string;
  resourceType?: string;
  result?: string;
}

export function normalizeAuditEventListResponse(payload: ApiResponse<PaginatedResult<AuditEventRecord>>): PaginatedResult<AuditEventRecord> {
  return {
    list: payload?.data?.list || [],
    total: payload?.data?.total || 0,
    page: payload?.data?.page || 1,
    pageSize: payload?.data?.pageSize || 10,
  };
}

export async function fetchAuditEvents(filters: AuditEventFilters = {}) {
  const response = await http.get('/audit/events', {
    ...withAuthHeaders(),
    params: {
      page: filters.page || 1,
      pageSize: filters.pageSize || 10,
      actor: filters.actor || undefined,
      action: filters.action || undefined,
      resourceType: filters.resourceType || undefined,
      result: filters.result || undefined,
    },
  });
  return normalizeAuditEventListResponse(response);
}

export async function fetchAuditEvent(id: number) {
  const response = await http.get<never, ApiResponse<AuditEventRecord | null>>(`/audit/events/${id}`, withAuthHeaders());
  return response.data;
}
