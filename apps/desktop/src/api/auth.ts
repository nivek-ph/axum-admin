import { bearerAuthorization, withAuthHeaders } from './core'
import { http } from './http'

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

export function fetchCaptcha() {
  return http.post('/auth/captcha') as Promise<{
    code: string
    message: string
    data?: CaptchaData
  }>
}

export function login(payload: LoginPayload) {
  return http.post('/auth/login', payload)
}

export function logout() {
  return http.post('/auth/logout', undefined, withAuthHeaders())
}

export function getUserInfo(token: string) {
  return http.get('/users/me', {
    headers: {
      Authorization: bearerAuthorization(token)
    }
  })
}

export function getMenu(token: string) {
  return http.get('/menus/current', {
    headers: {
      Authorization: bearerAuthorization(token)
    }
  })
}
