import type { AxiosAdapter } from 'axios'
import { render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, beforeEach, describe, expect, it } from 'vitest'

import { http } from '@/api/http'
import { MAX_UPLOAD_BYTES } from '@/api/files'
import { Application } from '@/app/Application'
import { useAuthStore } from '@/stores/auth'
import { useMenuStore } from '@/stores/menu'

describe('Files workflow', () => {
  const originalAdapter = http.defaults.adapter
  beforeEach(() => {
    useAuthStore.getState().clearSession()
    useMenuStore.getState().resetAccess()
    localStorage.clear()
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
    let uploads = 0
    let uploadCategory: unknown
    let uploadedName = ''
    let uploadedOffset = 0
    let finishUpload: (() => void) | undefined
    http.defaults.adapter = (async (config) => {
      let data: unknown
      if (config.url === '/users/me') data = { code: 'OK', message: 'ok', data: { userInfo: currentUser } }
      else if (config.url === '/menus/current')
        data = { code: 'OK', message: 'ok', data: { menus: [{ name: 'files', path: 'files' }], permissions: [] } }
      else if (config.url === '/files' && config.method === 'get') {
        listReads += 1
        data = { code: 'OK', message: 'ok', data: { list: [], total: 0, page: 1, pageSize: 10 } }
      } else if (config.url === '/files/uploads' && config.method === 'post') {
        uploads += 1
        const payload = JSON.parse(config.data as string) as { name: string; category: string; size: number }
        uploadCategory = payload.category
        uploadedName = payload.name
        data = { code: 'OK', message: 'ok', data: { id: 'upload-1', offset: 0, totalSize: payload.size, chunkSize: 3 } }
      } else if (config.url === '/files/uploads/upload-1' && config.method === 'patch') {
        uploadedOffset += (config.data as Blob).size
        if (uploadedOffset > 3) {
          await new Promise<void>((resolve) => {
            finishUpload = resolve
          })
        }
        data = {
          code: 'OK',
          message: 'ok',
          data: { id: 'upload-1', offset: uploadedOffset, totalSize: 6, chunkSize: 3 },
        }
      } else if (config.url === '/files/uploads/upload-1/complete' && config.method === 'post') {
        data = { code: 'OK', message: 'ok', data: {} }
      } else throw new Error(`Unexpected request: ${config.method} ${config.url}`)
      return { data, status: 200, statusText: 'OK', headers: {}, config }
    }) as AxiosAdapter
    window.history.replaceState({}, '', '/files')
    render(<Application />)

    await screen.findByRole('heading', { name: 'Manage uploads and external file URLs with flat metadata.' })
    await user.type(screen.getByLabelText('Filter by category'), 'documents')
    await user.click(screen.getByRole('button', { name: 'Search' }))
    await user.click(screen.getByRole('button', { name: 'Upload' }))
    expect(await screen.findByRole('heading', { name: 'Upload files' })).toBeVisible()
    const fileInput = screen.getByLabelText('Select files')
    expect(fileInput).toHaveAttribute('multiple')
    await user.upload(
      fileInput,
      [
        new File(['report'], 'report.txt', { type: 'text/plain' }),
        new File(['notes'], 'notes.txt', { type: 'text/plain' }),
      ],
    )
    expect(screen.getByText('report.txt')).toBeVisible()
    expect(screen.getByText('notes.txt')).toBeVisible()
    expect(screen.getAllByText('Pending')).toHaveLength(2)
    await user.click(screen.getByRole('button', { name: 'Clear' }))
    expect(screen.getByText('No files selected')).toBeVisible()
    await user.upload(fileInput, new File(['report'], 'report.txt', { type: 'text/plain' }))
    await user.click(screen.getByRole('button', { name: 'Start upload' }))

    await waitFor(() =>
      expect(screen.getByRole('progressbar', { name: 'report.txt' })).toHaveAttribute('aria-valuenow', '50'),
    )
    finishUpload?.()
    await waitFor(() => expect(uploadedName).toBe('report.txt'))
    expect(uploadCategory).toBe('documents')
    await waitFor(() => expect(listReads).toBeGreaterThan(2))
    const file = new File(['large'], 'large.bin')
    Object.defineProperty(file, 'size', { value: MAX_UPLOAD_BYTES + 1 })

    await user.upload(fileInput, file)

    expect(await screen.findByText('File is too large (maximum 1 GiB)')).toBeVisible()
    expect(uploads).toBe(1)
  })
})
