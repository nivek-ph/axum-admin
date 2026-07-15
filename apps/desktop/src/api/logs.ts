import { http } from './http';
import { withAuthHeaders, type ApiResponse } from './core';

export interface LoginLogRecord {
  id: number;
  username: string;
  ip: string;
  status: boolean;
  errorMessage: string;
  agent: string;
  createdAt: string;
}

export interface OperationLogRecord {
  id: number;
  ip: string;
  method: string;
  path: string;
  status: number;
  agent: string;
  errorMessage: string;
  body: string;
  resp: string;
  createdAt: string;
  user?: {
    userName: string;
    nickName: string;
  };
}

export interface PaginatedResult<T> {
  list: T[];
  total: number;
  page: number;
  pageSize: number;
}

export interface LoginLogFilters {
  page?: number;
  pageSize?: number;
  username?: string;
  status?: boolean;
}

export interface OperationLogFilters {
  page?: number;
  pageSize?: number;
  method?: string;
  path?: string;
  status?: number;
}

export function normalizeLoginLogListResponse(payload: ApiResponse<PaginatedResult<LoginLogRecord>>): PaginatedResult<LoginLogRecord> {
  return {
    list: payload?.data?.list || [],
    total: payload?.data?.total || 0,
    page: payload?.data?.page || 1,
    pageSize: payload?.data?.pageSize || 10,
  };
}

export function normalizeOperationLogListResponse(payload: ApiResponse<PaginatedResult<OperationLogRecord>>): PaginatedResult<OperationLogRecord> {
  return {
    list: payload?.data?.list || [],
    total: payload?.data?.total || 0,
    page: payload?.data?.page || 1,
    pageSize: payload?.data?.pageSize || 10,
  };
}

export async function fetchLoginLogs(filters: LoginLogFilters = {}) {
  const response = await http.get('/login-logs', {
    ...withAuthHeaders(),
    params: {
      page: filters.page || 1,
      pageSize: filters.pageSize || 10,
      username: filters.username || undefined,
      status: filters.status,
    },
  });
  return normalizeLoginLogListResponse(response);
}

export async function fetchOperationLogs(filters: OperationLogFilters = {}) {
  const response = await http.get('/operation-logs', {
    ...withAuthHeaders(),
    params: {
      page: filters.page || 1,
      pageSize: filters.pageSize || 10,
      method: filters.method || undefined,
      path: filters.path || undefined,
      status: filters.status,
    },
  });
  return normalizeOperationLogListResponse(response);
}
