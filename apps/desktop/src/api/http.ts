import axios, { AxiosHeaders } from 'axios'
import type { AxiosError, InternalAxiosRequestConfig } from 'axios'

import type { ApiEnvelope } from './core'
import { useAuthStore } from '@/stores/auth'
import { useMenuStore } from '@/stores/menu'

const defaultApiBaseUrl = 'http://127.0.0.1:3000/api'

export const http = axios.create({
  baseURL: import.meta.env.VITE_API_BASE_URL || defaultApiBaseUrl,
  timeout: 15_000,
})

export function isApiEnvelope(value: unknown): value is ApiEnvelope {
  if (!value || typeof value !== 'object') return false
  const candidate = value as Record<string, unknown>
  return typeof candidate.code === 'string' && typeof candidate.message === 'string'
}

export class ApiHttpError extends Error {
  readonly status?: number
  readonly body?: ApiEnvelope

  constructor(message: string, options?: { status?: number; body?: ApiEnvelope; cause?: unknown }) {
    super(message, options?.cause !== undefined ? { cause: options.cause } : undefined)
    this.name = 'ApiHttpError'
    this.status = options?.status
    this.body = options?.body
  }
}

type RetriableConfig = InternalAxiosRequestConfig & { _authRetried?: boolean }
interface TokenPair {
  accessToken: string
  refreshToken: string
}

let refreshInFlight: Promise<TokenPair> | null = null
const terminalCodes = new Set([
  'ACCESS_TOKEN_EXPIRED',
  'LOGIN_REQUIRED',
  'TOKEN_INVALID',
  'SESSION_INVALID',
  'REFRESH_TOKEN_INVALID',
  'USER_DISABLED',
])

function endLocalSession() {
  useAuthStore.getState().clearSession()
  useMenuStore.getState().resetAccess()
  if (typeof window !== 'undefined' && window.location.pathname !== '/login') {
    window.history.replaceState({}, '', '/login')
    window.dispatchEvent(new PopStateEvent('popstate'))
  }
}

async function refreshTokenPair() {
  if (!refreshInFlight) {
    refreshInFlight = (async () => {
      const response = await http.post<never, ApiEnvelope<TokenPair>>('/auth/refresh', {
        refreshToken: useAuthStore.getState().refreshToken,
      })
      if (response.code !== 'OK' || !response.data?.accessToken || !response.data.refreshToken) {
        throw new ApiHttpError(response.message || 'Request failed', { body: response })
      }
      useAuthStore.getState().setTokenPair(response.data.accessToken, response.data.refreshToken)
      return response.data
    })().finally(() => {
      refreshInFlight = null
    })
  }
  return refreshInFlight
}

function rejectedError(error: AxiosError) {
  const body = error.response?.data
  if (isApiEnvelope(body))
    return new ApiHttpError(body.message || 'Request failed', { status: error.response?.status, body, cause: error })
  return error
}

export function getApiErrorMessage(error: unknown, fallback: string) {
  if (axios.isAxiosError(error) && !error.response) return fallback
  if (error instanceof Error && error.message.trim()) return error.message
  return fallback
}

http.interceptors.response.use(
  (response) => response.data,
  async (error: AxiosError) => {
    const body = error.response?.data
    const config = error.config as RetriableConfig | undefined
    const url = config?.url ?? ''
    if (
      error.response?.status === 401 &&
      isApiEnvelope(body) &&
      body.code === 'ACCESS_TOKEN_EXPIRED' &&
      !url.includes('/auth/login') &&
      !url.includes('/auth/refresh') &&
      config &&
      !config._authRetried
    ) {
      try {
        const tokenPair = await refreshTokenPair()
        config._authRetried = true
        config.headers = AxiosHeaders.from(config.headers)
        config.headers.set('Authorization', `Bearer ${tokenPair.accessToken}`)
        return http.request(config)
      } catch (refreshError) {
        return Promise.reject(refreshError)
      }
    }
    if (isApiEnvelope(body) && terminalCodes.has(body.code) && !url.includes('/auth/login')) endLocalSession()
    return Promise.reject(rejectedError(error))
  },
)
