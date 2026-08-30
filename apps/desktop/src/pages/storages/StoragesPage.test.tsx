import type { AxiosAdapter } from 'axios'
import { cleanup, render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, beforeEach, describe, expect, it } from 'vitest'

import { http } from '@/api/http'
import { Application } from '@/app/Application'
import { useAuthStore } from '@/stores/auth'
import { useMenuStore } from '@/stores/menu'

describe('Storage workflow', () => {
  const originalAdapter = http.defaults.adapter

  beforeEach(() => {
    useAuthStore.getState().clearSession()
    useMenuStore.getState().resetAccess()
  })

  afterEach(() => {
    cleanup()
    http.defaults.adapter = originalAdapter
  })

  it('creates a local storage while keeping protected actions off the default card', async () => {
    const user = userEvent.setup()
    const currentUser = { id: 1, userName: 'admin', nickName: 'Admin' }
    useAuthStore.getState().setSession({ accessToken: 'token', refreshToken: 'refresh', userInfo: currentUser })
    let created: Record<string, unknown> | null = null
    let reads = 0
    http.defaults.adapter = (async (config) => {
      let data: unknown
      if (config.url === '/users/me') data = { code: 'OK', message: 'ok', data: { userInfo: currentUser } }
      else if (config.url === '/menus/current') {
        data = {
          code: 'OK',
          message: 'ok',
          data: {
            menus: [{ name: 'sys-storage', path: '/sys-storage' }],
            permissions: [
              'system:storage:create',
              'system:storage:update',
              'system:storage:delete',
              'system:storage:update-status',
              'system:storage:set-default',
            ],
          },
        }
      } else if (config.url === '/storages' && config.method === 'get') {
        reads += 1
        data = {
          code: 'OK',
          message: 'ok',
          data: {
            list: [
              {
                id: 1,
                name: 'Environment storage',
                code: 'environment',
                driver: 'local',
                root: './uploads',
                virtualHostStyle: false,
                hasAccessKey: false,
                hasSecretKey: false,
                enabled: true,
                isDefault: true,
                sort: 0,
                description: '',
                createdAt: '2026-08-24T00:00:00',
                updatedAt: '2026-08-24T00:00:00',
              },
            ],
          },
        }
      } else if (config.url === '/storages' && config.method === 'post') {
        created = JSON.parse(String(config.data))
        data = { code: 'OK', message: 'created', data: { id: 2, ...created } }
      } else throw new Error(`Unexpected request: ${config.method} ${config.url}`)
      return { data, status: 200, statusText: 'OK', headers: {}, config }
    }) as AxiosAdapter
    window.history.replaceState({}, '', '/sys-storage')
    render(<Application />)

    await screen.findByText('Environment storage')
    expect(screen.queryByRole('button', { name: 'Disable' })).not.toBeInTheDocument()
    expect(screen.queryByRole('button', { name: 'Delete' })).not.toBeInTheDocument()
    await user.click(screen.getByRole('button', { name: 'New storage' }))
    await user.type(screen.getByLabelText('Name'), 'Archive storage')
    await user.type(screen.getByLabelText('Code'), 'archive')
    await user.clear(screen.getByLabelText('Root directory'))
    await user.type(screen.getByLabelText('Root directory'), './archive')
    await user.click(screen.getByRole('button', { name: 'Save' }))

    await waitFor(() =>
      expect(created).toMatchObject({
        name: 'Archive storage',
        code: 'archive',
        driver: 'local',
        root: './archive',
      }),
    )
    await waitFor(() => expect(reads).toBeGreaterThan(1))
  })

  it('renders and edits an S3 storage from its discriminated response fields', async () => {
    const user = userEvent.setup()
    const currentUser = { id: 1, userName: 'admin', nickName: 'Admin' }
    useAuthStore.getState().setSession({ accessToken: 'token', refreshToken: 'refresh', userInfo: currentUser })
    http.defaults.adapter = (async (config) => {
      let data: unknown
      if (config.url === '/users/me') {
        data = { code: 'OK', message: 'ok', data: { userInfo: currentUser } }
      } else if (config.url === '/menus/current') {
        data = {
          code: 'OK',
          message: 'ok',
          data: {
            menus: [{ name: 'sys-storage', path: '/sys-storage' }],
            permissions: ['system:storage:update'],
          },
        }
      } else if (config.url === '/storages' && config.method === 'get') {
        data = {
          code: 'OK',
          message: 'ok',
          data: {
            list: [
              {
                id: 2,
                name: 'Archive objects',
                code: 'archive_objects',
                driver: 's3',
                root: 'uploads',
                bucket: 'archive-bucket',
                region: 'ap-southeast-1',
                endpoint: null,
                publicBaseUrl: 'https://cdn.example.test/uploads',
                virtualHostStyle: true,
                hasAccessKey: true,
                hasSecretKey: true,
                enabled: true,
                isDefault: false,
                sort: 10,
                description: 'Long-term archive',
                createdAt: '2026-08-30T00:00:00',
                updatedAt: '2026-08-30T00:00:00',
              },
            ],
          },
        }
      } else {
        throw new Error(`Unexpected request: ${config.method} ${config.url}`)
      }
      return { data, status: 200, statusText: 'OK', headers: {}, config }
    }) as AxiosAdapter
    window.history.replaceState({}, '', '/sys-storage')
    render(<Application />)

    await screen.findByText('Archive objects')
    expect(screen.getByText('archive-bucket')).toBeInTheDocument()
    expect(screen.getByText('AWS')).toBeInTheDocument()
    await user.click(screen.getByRole('button', { name: 'Edit' }))

    expect(screen.getByLabelText('Bucket')).toHaveValue('archive-bucket')
    expect(screen.getByLabelText('Region')).toHaveValue('ap-southeast-1')
    expect(screen.getByLabelText('Root path')).toHaveValue('uploads')
    expect(screen.getByLabelText('Public URL')).toHaveValue('https://cdn.example.test/uploads')
    expect(screen.getByLabelText('Access key')).toHaveAttribute('placeholder', 'Leave blank to keep current')
    expect(screen.getByLabelText('Secret key')).toHaveAttribute('placeholder', 'Leave blank to keep current')
    expect(screen.getByLabelText('Virtual-hosted style')).toBeChecked()
  })
})
