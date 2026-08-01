import { getCurrentMenu, getUserInfo } from '@/api/auth'
import { isLoginRequiredError } from '@/api/http'
import { isAuthenticated, useAuthStore } from '@/stores/auth'
import { useMenuStore } from '@/stores/menu'

export async function bootstrapAuthSession() {
  const auth = useAuthStore.getState()
  if (!isAuthenticated(auth)) {
    useMenuStore.getState().resetAccess()
    return
  }
  try {
    const [userResponse, menuResponse] = await Promise.all([
      getUserInfo(auth.accessToken),
      getCurrentMenu(auth.accessToken),
    ])
    const user = userResponse.data?.userInfo
    if (userResponse.code !== 'OK' || menuResponse.code !== 'OK' || !user) throw new Error('Invalid session')
    const permissions = menuResponse.data?.permissions ?? []
    useAuthStore.getState().setUserAndPermissions(user, permissions)
    useMenuStore.getState().setAuthorizedMenus(menuResponse.data?.menus ?? [])
  } catch (error) {
    if (isLoginRequiredError(error)) {
      useAuthStore.getState().clearSession()
      useMenuStore.getState().resetAccess()
      return
    }
    useMenuStore.getState().setAuthorizedMenus([])
  }
}
