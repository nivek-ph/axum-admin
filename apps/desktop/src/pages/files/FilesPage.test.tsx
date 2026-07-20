import type { AxiosAdapter } from 'axios'
import { render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, beforeEach, describe, expect, it } from 'vitest'

import { http } from '@/api/http'
import { Application } from '@/app/Application'
import { useAuthStore } from '@/stores/auth'
import { useMenuStore } from '@/stores/menu'

describe('Files workflow', () => {
  const originalAdapter = http.defaults.adapter
  beforeEach(() => {
    useAuthStore.getState().clearSession()
    useMenuStore.getState().resetAccess()
  })
  afterEach(() => {
    http.defaults.adapter = originalAdapter
  })

  it('uploads a selected file with the active category and refreshes the library', async () => {
    const user = userEvent.setup()
    const currentUser = {
      id: 1,
      userName: 'admin',
      nickName: 'Admin',
      roles: [{ id: 1, code: 'super_admin', name: 'Super Admin' }],
    }
    useAuthStore.getState().setSession({ accessToken: 'token', refreshToken: 'refresh', userInfo: currentUser })
    let listReads = 0
    let uploadCategory: unknown
    let uploadedName = ''
    http.defaults.adapter = (async (config) => {
      let data: unknown
      if (config.url === '/users/me') data = { code: 'OK', message: 'ok', data: { userInfo: currentUser } }
      else if (config.url === '/menus/current')
        data = { code: 'OK', message: 'ok', data: { menus: [{ name: 'files', path: 'files' }], permissions: [] } }
      else if (config.url === '/files' && config.method === 'get') {
        listReads += 1
        data = { code: 'OK', message: 'ok', data: { list: [], total: 0, page: 1, pageSize: 10 } }
      } else if (config.url === '/files/upload') {
        uploadCategory = config.params?.category
        uploadedName =
          (config.data as FormData).get('file') instanceof File
            ? ((config.data as FormData).get('file') as File).name
            : ''
        data = { code: 'OK', message: 'ok', data: {} }
      } else throw new Error(`Unexpected request: ${config.method} ${config.url}`)
      return { data, status: 200, statusText: 'OK', headers: {}, config }
    }) as AxiosAdapter
    window.history.replaceState({}, '', '/files')
    const { container } = render(<Application />)

    await screen.findByRole('heading', { name: 'Files' })
    await user.type(screen.getByLabelText('Filter by category'), 'documents')
    await user.click(screen.getByRole('button', { name: 'Search' }))
    await user.upload(
      container.querySelector('input[type="file"]') as HTMLInputElement,
      new File(['report'], 'report.txt', { type: 'text/plain' }),
    )

    await waitFor(() => expect(uploadedName).toBe('report.txt'))
    expect(uploadCategory).toBe('documents')
    await waitFor(() => expect(listReads).toBeGreaterThan(2))
  })
})
