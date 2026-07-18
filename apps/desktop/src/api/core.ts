import { useAuthStore } from '@/stores/auth';

export interface ApiResponse<T = any> {
  code: string;
  data: T | null;
  message: string;
}

export const API_OK = 'OK';

export function bearerAuthorization(token: string) {
  const trimmed = token.trim();
  return trimmed ? `Bearer ${trimmed}` : '';
}

export function withAuthHeaders() {
  const authStore = useAuthStore();
  return {
    headers: {
      Authorization: bearerAuthorization(authStore.accessToken),
    },
  };
}
