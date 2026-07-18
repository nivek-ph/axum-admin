import axios, { AxiosHeaders } from 'axios';
import type { AxiosError, InternalAxiosRequestConfig } from 'axios';

import { useAuthStore } from '@/stores/auth';
import { clearAuthSession, readAuthSession } from '@/stores/authStorage';
import { useMenuStore } from '@/stores/menu';
import { ElMessage } from '@/ui/feedback';

const defaultApiBaseUrl = 'http://127.0.0.1:3000/api';

export const http = axios.create({
  baseURL: import.meta.env.VITE_API_BASE_URL || defaultApiBaseUrl,
  timeout: 15_000,
});

export interface ApiEnvelope {
  code: string;
  message: string;
  data?: unknown;
}

export function isApiEnvelope(value: unknown): value is ApiEnvelope {
  if (typeof value !== 'object' || value === null) return false;
  const o = value as Record<string, unknown>;
  return typeof o.code === 'string' && typeof o.message === 'string';
}

export class ApiHttpError extends Error {
  readonly status?: number;
  readonly body?: ApiEnvelope;

  constructor(message: string, opts?: { status?: number; body?: ApiEnvelope; cause?: unknown }) {
    super(message, opts?.cause !== undefined ? { cause: opts.cause } : undefined);
    this.name = 'ApiHttpError';
    this.status = opts?.status;
    this.body = opts?.body;
  }
}

interface TokenPairData {
  accessToken: string;
  refreshToken: string;
}

type RetriableRequestConfig = InternalAxiosRequestConfig & {
  _authRetried?: boolean;
};

let refreshInFlight: Promise<TokenPairData> | null = null;
const terminalAuthenticationCodes = new Set([
  'ACCESS_TOKEN_EXPIRED',
  'LOGIN_REQUIRED',
  'TOKEN_INVALID',
  'SESSION_INVALID',
  'REFRESH_TOKEN_INVALID',
  'USER_DISABLED',
]);

function asRejectedError(error: AxiosError) {
  const status = error.response?.status;
  const data = error.response?.data;
  if (isApiEnvelope(data)) {
    const msg = data.message?.trim() ? data.message : 'Request failed';
    return new ApiHttpError(msg, { status, body: data, cause: error });
  }
  return error;
}

function endLocalSession() {
  let hadSession = false;
  try {
    const authStore = useAuthStore();
    hadSession = authStore.isAuthenticated;
    authStore.clearSession();
    useMenuStore().resetAccess();
  } catch {
    const persisted = readAuthSession();
    hadSession = Boolean(persisted.accessToken && persisted.refreshToken);
    clearAuthSession();
  }

  if (hadSession) {
    ElMessage.warning('Server session may still be active');
  }
  if (typeof window !== 'undefined' && !window.location.hash.includes('/login')) {
    window.location.hash = '#/login';
  }
}

async function refreshTokenPair(): Promise<TokenPairData> {
  if (!refreshInFlight) {
    refreshInFlight = (async () => {
      const authStore = useAuthStore();
      const response = await http.post('/auth/refresh', {
        refreshToken: authStore.refreshToken,
      }) as ApiEnvelope;
      const data = response.data as Partial<TokenPairData> | undefined;
      if (
        response.code !== 'OK'
        || typeof data?.accessToken !== 'string'
        || typeof data?.refreshToken !== 'string'
        || !data.accessToken
        || !data.refreshToken
      ) {
        throw new ApiHttpError(response.message || 'Request failed', { body: response });
      }
      authStore.setTokenPair(data.accessToken, data.refreshToken);
      return {
        accessToken: data.accessToken,
        refreshToken: data.refreshToken,
      };
    })().finally(() => {
      refreshInFlight = null;
    });
  }
  return refreshInFlight;
}

/** Prefer backend `{ message }`; otherwise fallback (e.g. network / non-JSON error). */
export function getApiErrorMessage(err: unknown, fallback: string): string {
  if (err instanceof ApiHttpError) {
    const m = err.message?.trim();
    return m ? m : fallback;
  }
  if (axios.isAxiosError(err)) {
    const data = err.response?.data;
    if (isApiEnvelope(data)) {
      const m = data.message?.trim();
      if (m) return m;
    }
  }
  if (err instanceof Error) {
    const m = err.message?.trim();
    if (m) return m;
  }
  return fallback;
}

http.interceptors.response.use(
  (response) => response.data,
  async (error: AxiosError) => {
    const status = error.response?.status;
    const data = error.response?.data;
    const requestUrl = error.config?.url || '';
    const isLoginRequest = requestUrl.includes('/auth/login');
    const isRefreshRequest = requestUrl.includes('/auth/refresh');
    const requestConfig = error.config as RetriableRequestConfig | undefined;

    if (
      status === 401
      && isApiEnvelope(data)
      && data.code === 'ACCESS_TOKEN_EXPIRED'
      && !isLoginRequest
      && !isRefreshRequest
      && requestConfig
      && !requestConfig._authRetried
    ) {
      try {
        const pair = await refreshTokenPair();
        requestConfig._authRetried = true;
        requestConfig.headers = AxiosHeaders.from(requestConfig.headers);
        requestConfig.headers.set('Authorization', `Bearer ${pair.accessToken}`);
        return http.request(requestConfig);
      } catch (refreshError) {
        return Promise.reject(refreshError);
      }
    }

    if (
      isApiEnvelope(data)
      && terminalAuthenticationCodes.has(data.code)
      && !isLoginRequest
    ) {
      endLocalSession();
    }

    return Promise.reject(asRejectedError(error));
  }
);
