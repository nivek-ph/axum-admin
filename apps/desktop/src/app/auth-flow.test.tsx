import { AxiosError, type AxiosAdapter, type AxiosRequestConfig, type InternalAxiosRequestConfig } from 'axios'
import { cleanup, render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, describe, expect, it, vi } from 'vitest'

import { http } from '@/api/http'
import { Application } from '@/app/Application'
import { useAuthStore } from '@/stores/auth'
import { useMenuStore } from '@/stores/menu'

function response(config: AxiosRequestConfig, data: unknown) {
  return Promise.resolve({
    data,
    status: 200,
    statusText: 'OK',
    headers: {},
    config,
  })
}

function rejectEnvelope(config: InternalAxiosRequestConfig, code: string, status = 401) {
  return Promise.reject(
    new AxiosError('request failed', String(status), config, undefined, {
      data: { code, message: code },
      status,
      statusText: 'Error',
      headers: {},
      config,
    }),
  )
}

describe('Admin Console authentication', () => {
  const originalAdapter = http.defaults.adapter

  afterEach(() => {
    cleanup()
    http.defaults.adapter = originalAdapter
    useAuthStore.getState().clearSession()
    useMenuStore.getState().resetAccess()
  })

  it('shows visible validation messages for an empty sign-in form', async () => {
    http.defaults.adapter = (async (config) =>
      response(config, {
        code: 'OK',
        message: 'ok',
        data: { openCaptcha: false },
      })) as AxiosAdapter

    window.history.replaceState({}, '', '/login')
    render(<Application />)

    await userEvent.setup().click(await screen.findByRole('button', { name: 'Sign in' }))

    expect(await screen.findByText('Username is required')).toBeInTheDocument()
    expect(screen.getByText('Password is required')).toBeInTheDocument()
    expect(screen.getByLabelText('Username')).toHaveAttribute('aria-invalid', 'true')
    expect(screen.getByLabelText('Password')).toHaveAttribute('aria-invalid', 'true')
  })

  it('requires captcha locally when captcha is enabled', async () => {
    let loginCalls = 0
    http.defaults.adapter = (async (config) => {
      if (config.url === '/auth/captcha') {
        return response(config, {
          code: 'OK',
          message: 'ok',
          data: {
            captchaLength: 4,
            picPath: 'data:image/svg+xml;base64,PHN2Zy8+',
            captchaId: 'captcha-123',
            openCaptcha: true,
          },
        })
      }
      if (config.url === '/auth/login') loginCalls += 1
      throw new Error(`Unexpected request: ${config.method} ${config.url}`)
    }) as AxiosAdapter

    window.history.replaceState({}, '', '/login')
    render(<Application />)

    await screen.findByLabelText('Captcha')
    await userEvent.setup().click(screen.getByRole('button', { name: 'Sign in' }))

    expect(await screen.findByText('Captcha is required')).toBeInTheDocument()
    expect(screen.getByText('Username is required')).toBeInTheDocument()
    expect(screen.getByText('Password is required')).toBeInTheDocument()
    expect(screen.getByLabelText('Captcha')).toHaveAttribute('aria-invalid', 'true')
    expect(loginCalls).toBe(0)
  })

  it('starts only one captcha refresh while a refresh is already in flight', async () => {
    let captchaCalls = 0
    let releaseRefresh!: () => void
    const refreshBarrier = new Promise<void>((resolve) => {
      releaseRefresh = resolve
    })
    http.defaults.adapter = (async (config) => {
      if (config.url !== '/auth/captcha') throw new Error(`Unexpected request: ${config.method} ${config.url}`)
      captchaCalls += 1
      if (captchaCalls > 1) await refreshBarrier
      return response(config, {
        code: 'OK',
        message: 'ok',
        data: {
          captchaLength: 4,
          picPath: 'data:image/svg+xml;base64,PHN2Zy8+',
          captchaId: `captcha-${captchaCalls}`,
          openCaptcha: true,
        },
      })
    }) as AxiosAdapter

    window.history.replaceState({}, '', '/login')
    render(<Application />)

    const reload = await screen.findByRole('button', { name: 'Reload captcha' })
    const user = userEvent.setup()
    const firstRefresh = user.click(reload)
    await vi.waitFor(() => expect(captchaCalls).toBe(2))

    expect(reload).toBeDisabled()
    await user.click(reload)
    expect(captchaCalls).toBe(2)

    releaseRefresh()
    await firstRefresh
  })

  it('signs in and opens the authorized home route', async () => {
    http.defaults.adapter = (async (config) => {
      if (config.url === '/auth/captcha') {
        return response(config, {
          code: 'OK',
          message: 'ok',
          data: {
            captchaLength: 4,
            picPath: 'data:image/svg+xml;base64,PHN2Zy8+',
            captchaId: 'captcha-123',
            openCaptcha: true,
          },
        })
      }
      if (config.url === '/auth/login') {
        return response(config, {
          code: 'OK',
          message: 'ok',
          data: {
            accessToken: 'access-token',
            refreshToken: 'refresh-token',
            user: {
              id: 7,
              userName: 'operator',
              nickName: 'Operator',
              homeRoute: 'dashboard',
              roles: [{ id: 7, code: 'operator', name: 'Operator' }],
            },
          },
        })
      }
      if (config.url === '/users/me') {
        return response(config, {
          code: 'OK',
          message: 'ok',
          data: {
            userInfo: {
              id: 7,
              userName: 'operator',
              nickName: 'Operator',
              homeRoute: 'dashboard',
              roles: [{ id: 7, code: 'operator', name: 'Operator' }],
            },
          },
        })
      }
      if (config.url === '/menus/current') {
        return response(config, {
          code: 'OK',
          message: 'ok',
          data: {
            menus: [
              { name: 'dashboard', path: 'dashboard', meta: { title: 'Dashboard' } },
              { name: 'users', path: 'users', meta: { title: 'Users' } },
            ],
            permissions: ['system:user:list'],
          },
        })
      }
      throw new Error(`Unexpected request: ${config.method} ${config.url}`)
    }) as AxiosAdapter

    window.history.replaceState({}, '', '/login')
    render(<Application />)

    const user = userEvent.setup()
    expect(await screen.findByRole('img', { name: 'Captcha' })).toBeInTheDocument()
    await user.type(screen.getByLabelText('Username'), 'operator')
    await user.type(screen.getByLabelText('Password'), 'secret')
    await user.type(screen.getByLabelText('Captcha'), 'abcd')
    await user.click(screen.getByRole('button', { name: 'Sign in' }))

    expect(await screen.findByRole('heading', { name: 'Welcome back, Operator.' })).toBeInTheDocument()
    expect(screen.getByRole('navigation', { name: 'Main navigation' }).querySelector('a[href="/users"]')).not.toBeNull()
    expect(JSON.parse(localStorage.getItem('axum-vue-admin.auth') || '{}')).toMatchObject({
      accessToken: 'access-token',
      refreshToken: 'refresh-token',
    })
  })

  it('does not publish an authenticated shell before access bootstrap completes', async () => {
    let releaseBootstrap!: () => void
    const bootstrapBarrier = new Promise<void>((resolve) => {
      releaseBootstrap = resolve
    })
    let bootstrapCalls = 0
    http.defaults.adapter = (async (config) => {
      if (config.url === '/auth/captcha') {
        return response(config, { code: 'OK', message: 'ok', data: { openCaptcha: false } })
      }
      if (config.url === '/auth/login') {
        return response(config, {
          code: 'OK',
          message: 'ok',
          data: {
            accessToken: 'access-token',
            refreshToken: 'refresh-token',
            user: { id: 7, userName: 'operator', nickName: 'Operator', homeRoute: 'dashboard' },
          },
        })
      }
      if (config.url === '/users/me' || config.url === '/menus/current') {
        bootstrapCalls += 1
        await bootstrapBarrier
        return response(
          config,
          config.url === '/users/me'
            ? {
                code: 'OK',
                message: 'ok',
                data: { userInfo: { id: 7, userName: 'operator', nickName: 'Operator', homeRoute: 'dashboard' } },
              }
            : {
                code: 'OK',
                message: 'ok',
                data: { menus: [{ name: 'dashboard', path: 'dashboard' }], permissions: [] },
              },
        )
      }
      throw new Error(`Unexpected request: ${config.method} ${config.url}`)
    }) as AxiosAdapter

    window.history.replaceState({}, '', '/login')
    render(<Application />)

    const user = userEvent.setup()
    await user.type(await screen.findByLabelText('Username'), 'operator')
    await user.type(screen.getByLabelText('Password'), 'secret')
    await user.click(screen.getByRole('button', { name: 'Sign in' }))
    await vi.waitFor(() => expect(bootstrapCalls).toBe(2))

    expect(screen.queryByRole('navigation', { name: 'Main navigation' })).not.toBeInTheDocument()
    expect(localStorage.getItem('axum-vue-admin.auth')).toBeNull()
    expect(useAuthStore.getState()).toMatchObject({
      accessToken: 'access-token',
      refreshToken: 'refresh-token',
      userInfo: null,
    })

    releaseBootstrap()
    expect(await screen.findByRole('heading', { name: 'Welcome back, Operator.' })).toBeInTheDocument()
  })

  it('keeps a refreshed token pair when post-login access bootstrap refreshes the access token', async () => {
    let refreshCalls = 0
    http.defaults.adapter = (async (config) => {
      if (config.url === '/auth/captcha') {
        return response(config, { code: 'OK', message: 'ok', data: { openCaptcha: false } })
      }
      if (config.url === '/auth/login') {
        return response(config, {
          code: 'OK',
          message: 'ok',
          data: {
            accessToken: 'access-old',
            refreshToken: 'refresh-old',
            user: { id: 7, userName: 'operator', nickName: 'Operator', homeRoute: 'dashboard' },
          },
        })
      }
      if (config.url === '/auth/refresh') {
        refreshCalls += 1
        expect(JSON.parse(String(config.data))).toEqual({ refreshToken: 'refresh-old' })
        return response(config, {
          code: 'OK',
          message: 'ok',
          data: { accessToken: 'access-new', refreshToken: 'refresh-new' },
        })
      }
      if (config.url === '/users/me' || config.url === '/menus/current') {
        if (String(config.headers.get('Authorization')) === 'Bearer access-old') {
          return rejectEnvelope(config, 'ACCESS_TOKEN_EXPIRED')
        }
        expect(String(config.headers.get('Authorization'))).toBe('Bearer access-new')
        return response(
          config,
          config.url === '/users/me'
            ? {
                code: 'OK',
                message: 'ok',
                data: { userInfo: { id: 7, userName: 'operator', nickName: 'Operator', homeRoute: 'dashboard' } },
              }
            : {
                code: 'OK',
                message: 'ok',
                data: { menus: [{ name: 'dashboard', path: 'dashboard' }], permissions: [] },
              },
        )
      }
      throw new Error(`Unexpected request: ${config.method} ${config.url}`)
    }) as AxiosAdapter

    window.history.replaceState({}, '', '/login')
    render(<Application />)

    const user = userEvent.setup()
    await user.type(await screen.findByLabelText('Username'), 'operator')
    await user.type(screen.getByLabelText('Password'), 'secret')
    await user.click(screen.getByRole('button', { name: 'Sign in' }))

    expect(await screen.findByRole('heading', { name: 'Welcome back, Operator.' })).toBeInTheDocument()
    expect(refreshCalls).toBe(1)
    expect(JSON.parse(localStorage.getItem('axum-vue-admin.auth') || '{}')).toMatchObject({
      accessToken: 'access-new',
      refreshToken: 'refresh-new',
    })
  })
})
