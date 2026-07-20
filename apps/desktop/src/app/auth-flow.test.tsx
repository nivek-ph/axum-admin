import type { AxiosAdapter, AxiosRequestConfig } from 'axios'
import { cleanup, render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, describe, expect, it } from 'vitest'

import { http } from '@/api/http'
import { Application } from '@/app/Application'

function response(config: AxiosRequestConfig, data: unknown) {
  return Promise.resolve({
    data,
    status: 200,
    statusText: 'OK',
    headers: {},
    config,
  })
}

describe('Admin Console authentication', () => {
  const originalAdapter = http.defaults.adapter

  afterEach(() => {
    cleanup()
    http.defaults.adapter = originalAdapter
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

    expect(await screen.findByRole('heading', { name: 'Dashboard' })).toBeInTheDocument()
    expect(screen.getByRole('link', { name: 'Users' })).toBeInTheDocument()
    expect(JSON.parse(localStorage.getItem('axum-vue-admin.auth') || '{}')).toMatchObject({
      accessToken: 'access-token',
      refreshToken: 'refresh-token',
    })
  })
})
