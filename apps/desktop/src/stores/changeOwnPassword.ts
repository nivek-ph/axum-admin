import {
  changeOwnPassword,
  type ChangeOwnPasswordPayload,
} from '@/api/users'

import { useAuthStore } from './auth'
import { useMenuStore } from './menu'

export async function changeOwnPasswordAndSignOut(
  payload: ChangeOwnPasswordPayload,
  navigateToLogin: () => Promise<unknown> | unknown,
) {
  await changeOwnPassword(payload)
  useAuthStore().clearSession()
  useMenuStore().resetAccess()
  await navigateToLogin()
}
