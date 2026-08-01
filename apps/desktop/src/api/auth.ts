import type { ApiResponse } from './core'
import { bearerAuthorization } from './core'
import { http } from './http'
import type { AuthUserInfo } from '@/stores/auth'
import { useAuthStore } from '@/stores/auth'
import type { RemoteMenuItem } from '@/stores/menu'

export interface LoginPayload {
  username: string
  password: string
  captcha: string
  captchaId: string
}

export interface CaptchaData {
  captchaLength: number
  picPath: string
  captchaId: string
  openCaptcha: boolean
}

export interface LoginData {
  accessToken: string
  refreshToken: string
  user: AuthUserInfo
}

export function fetchCaptcha() {
  return http.post<never, ApiResponse<CaptchaData>>('/auth/captcha')
}

export function login(payload: LoginPayload) {
  return http.post<never, ApiResponse<LoginData>>('/auth/login', payload)
}

export function logout() {
  return http.post<never, ApiResponse<null>>('/auth/logout', undefined, {
    headers: { Authorization: bearerAuthorization(useAuthStore.getState().accessToken) },
  })
}

export function getUserInfo(token: string) {
  return http.get<never, ApiResponse<{ userInfo?: AuthUserInfo }>>('/users/me', {
    headers: { Authorization: bearerAuthorization(token) },
  })
}

export function getCurrentMenu(token: string) {
  return http.get<never, ApiResponse<{ menus?: RemoteMenuItem[]; permissions?: string[] }>>('/menus/current', {
    headers: { Authorization: bearerAuthorization(token) },
  })
}
