import { useAuthStore } from '@/stores/auth'

export const API_OK = 'OK'

export interface ApiEnvelope<T = unknown> {
  code: string
  message: string
  data?: T
}

export function bearerAuthorization(token: string) {
  return token.trim() ? `Bearer ${token.trim()}` : ''
}

export function withAuthHeaders() {
  return { headers: { Authorization: bearerAuthorization(useAuthStore.getState().accessToken) } }
}
