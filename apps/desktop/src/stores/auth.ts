import { create } from 'zustand'

export const AUTH_STORAGE_KEY = 'axum-vue-admin.auth'

export interface AuthRole {
  id: number
  code: string
  name: string
}

export interface AuthUserInfo {
  id: number
  userName: string
  nickName: string
  headerImg?: string
  homeRoute?: string
  phone?: string
  email?: string
  deptId?: number | null
  deptName?: string
  roles?: AuthRole[]
  roleIds?: number[]
  permissions?: string[]
}

export interface AuthSession {
  accessToken: string
  refreshToken: string
  userInfo: AuthUserInfo | null
}

function emptySession(): AuthSession {
  return { accessToken: '', refreshToken: '', userInfo: null }
}

function isUserInfo(value: unknown): value is AuthUserInfo {
  if (!value || typeof value !== 'object') return false
  const user = value as Record<string, unknown>
  return (
    Number.isFinite(user.id) &&
    typeof user.userName === 'string' &&
    user.userName.trim().length > 0 &&
    typeof user.nickName === 'string'
  )
}

export function readAuthSession(): AuthSession {
  if (typeof localStorage === 'undefined') return emptySession()
  try {
    const value = localStorage.getItem(AUTH_STORAGE_KEY)
    if (!value) return emptySession()
    const parsed = JSON.parse(value) as Partial<AuthSession>
    const accessToken = typeof parsed.accessToken === 'string' ? parsed.accessToken.trim() : ''
    const refreshToken = typeof parsed.refreshToken === 'string' ? parsed.refreshToken.trim() : ''
    const userInfo = isUserInfo(parsed.userInfo) ? parsed.userInfo : null
    if (!accessToken || !refreshToken || !userInfo) {
      localStorage.removeItem(AUTH_STORAGE_KEY)
      return emptySession()
    }
    return { accessToken, refreshToken, userInfo }
  } catch {
    localStorage.removeItem(AUTH_STORAGE_KEY)
    return emptySession()
  }
}

function persistSession(session: AuthSession) {
  if (!session.accessToken || !session.refreshToken || !session.userInfo) {
    localStorage.removeItem(AUTH_STORAGE_KEY)
    return
  }
  localStorage.setItem(AUTH_STORAGE_KEY, JSON.stringify(session))
}

interface AuthState extends AuthSession {
  permissions: string[]
  setSession: (session: AuthSession) => void
  setTokenPair: (accessToken: string, refreshToken: string) => void
  setUserAndPermissions: (userInfo: AuthUserInfo, permissions: string[]) => void
  clearSession: () => void
  can: (permission: string) => boolean
}

const initialSession = readAuthSession()

export const useAuthStore = create<AuthState>((set, get) => ({
  ...initialSession,
  permissions: initialSession.userInfo?.permissions ?? [],
  setSession: (session) => {
    persistSession(session)
    set({ ...session, permissions: session.userInfo?.permissions ?? [] })
  },
  setTokenPair: (accessToken, refreshToken) => {
    const session = { accessToken, refreshToken, userInfo: get().userInfo }
    persistSession(session)
    set({ accessToken, refreshToken })
  },
  setUserAndPermissions: (userInfo, permissions) => {
    const nextUser = { ...userInfo, permissions }
    const session = {
      accessToken: get().accessToken,
      refreshToken: get().refreshToken,
      userInfo: nextUser,
    }
    persistSession(session)
    set({ userInfo: nextUser, permissions })
  },
  clearSession: () => {
    localStorage.removeItem(AUTH_STORAGE_KEY)
    set({ ...emptySession(), permissions: [] })
  },
  can: (permission) => {
    const state = get()
    if (state.userInfo?.roles?.some((role) => role.code === 'super_admin')) return true
    return state.permissions.includes(permission)
  },
}))

export function isAuthenticated(state: AuthSession) {
  return Boolean(state.accessToken && state.refreshToken && state.userInfo)
}

export function isSuperAdmin(userInfo: AuthUserInfo | null) {
  return Boolean(userInfo?.roles?.some((role) => role.code === 'super_admin'))
}
