import type { AxiosAdapter } from 'axios'
import { QueryClientProvider } from '@tanstack/react-query'
import { cleanup, render, screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { MemoryRouter } from 'react-router-dom'
import { afterEach, beforeEach, describe, expect, it } from 'vitest'

import { http } from '@/api/http'
import i18n from '@/i18n'
import { createQueryClient } from '@/lib/query'
import { DashboardPage } from '@/pages/DashboardPage'
import { useAuthStore } from '@/stores/auth'
import { useMenuStore } from '@/stores/menu'

const RESOURCE_RESPONSES: Record<string, unknown> = {
  '/users': { list: [], total: 12, page: 1, pageSize: 1 },
  '/roles': { list: [{ id: 1 }, { id: 2 }] },
  '/depts': { list: [{ id: 1, children: [{ id: 2 }] }] },
  '/files': { list: [], total: 4, page: 1, pageSize: 1 },
  '/params': { list: [], total: 6, page: 1, pageSize: 1 },
  '/dictionaries': [{ id: 1 }, { id: 2 }, { id: 3 }],
}

const auditStats = {
  days: 14,
  eventCount: 9,
  todayLogins: 2,
  todayIps: 1,
  daily: [
    {
      date: '2026-07-24',
      logins: 0,
      ips: 0,
      loginFailures: 1,
      accessDenials: 0,
    },
    {
      date: '2026-07-25',
      logins: 2,
      ips: 1,
      loginFailures: 1,
      accessDenials: 1,
    },
  ],
}

function setSession(superAdmin = true) {
  useAuthStore.getState().setSession({
    accessToken: 'token',
    refreshToken: 'refresh',
    userInfo: {
      id: 1,
      userName: 'admin',
      nickName: 'Admin',
      roles: [{ id: 1, code: superAdmin ? 'super_admin' : 'operator', name: 'Operator' }],
    },
  })
}

function successAdapter(statsData = auditStats, requests: string[] = [], requestedDays: unknown[] = []): AxiosAdapter {
  return (async (config) => {
    const url = config.url ?? ''
    requests.push(url)
    if (url === '/audit/events/stats') requestedDays.push(config.params?.days)
    const response = url === '/audit/events/stats' ? statsData : RESOURCE_RESPONSES[url]
    if (response === undefined) throw new Error(`Unexpected request: ${config.method} ${url}`)
    return {
      data: { code: 'OK', message: 'ok', data: response },
      status: 200,
      statusText: 'OK',
      headers: {},
      config,
    }
  }) as AxiosAdapter
}

function renderDashboard() {
  return render(
    <QueryClientProvider client={createQueryClient()}>
      <MemoryRouter>
        <DashboardPage />
      </MemoryRouter>
    </QueryClientProvider>,
  )
}

describe('Dashboard page', () => {
  const originalAdapter = http.defaults.adapter

  beforeEach(async () => {
    useAuthStore.getState().clearSession()
    useMenuStore.getState().resetAccess()
    await i18n.changeLanguage('en-US')
  })

  afterEach(() => {
    cleanup()
    http.defaults.adapter = originalAdapter
  })

  it('puts login and security trends before resource cards and switches login chart mode locally', async () => {
    const requests: string[] = []
    const requestedDays: unknown[] = []
    setSession()
    useMenuStore.getState().setAuthorizedMenus([], true)
    http.defaults.adapter = successAdapter(auditStats, requests, requestedDays)

    const view = renderDashboard()

    expect(screen.getByRole('heading', { name: 'Welcome back, Admin.' })).toBeInTheDocument()
    const loginMetric = await screen.findByLabelText('Logins today')
    await waitFor(() => expect(within(loginMetric).getByText('2')).toBeInTheDocument())
    expect(loginMetric.closest('a, button')).toBeNull()
    expect(within(screen.getByLabelText('IPs today')).getByText('1')).toBeInTheDocument()
    const loginTrend = screen.getByText('Login trend')
    expect(screen.getByText('Security event trend')).toBeInTheDocument()
    expect(screen.getAllByText('Last 14 days · UTC')).toHaveLength(2)
    const loginSeries = screen.getByRole('group', { name: 'Login trend series' })
    expect(within(loginSeries).getByText('Logins')).toBeInTheDocument()
    expect(within(loginSeries).getByText('IPs')).toBeInTheDocument()
    const securitySeries = screen.getByRole('group', { name: 'Security event trend series' })
    expect(within(securitySeries).getByText('Login failures')).toBeInTheDocument()
    expect(within(securitySeries).getByText('Access denials')).toBeInTheDocument()
    expect(screen.getByLabelText('Login trend dates: 07-24, 07-25')).toBeInTheDocument()
    const usersLink = await screen.findByRole('link', { name: 'Users: 12' })
    expect(loginTrend.compareDocumentPosition(usersLink) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy()
    expect(screen.queryByText('Quick functions')).not.toBeInTheDocument()
    expect(screen.queryByText('Popular modules (Top 10)')).not.toBeInTheDocument()

    const areaButton = screen.getByRole('button', { name: 'Area' })
    const lineButton = screen.getByRole('button', { name: 'Line' })
    expect(areaButton).toHaveAttribute('aria-pressed', 'true')
    expect(lineButton).toHaveAttribute('aria-pressed', 'false')
    await userEvent.click(lineButton)
    expect(areaButton).toHaveAttribute('aria-pressed', 'false')
    expect(lineButton).toHaveAttribute('aria-pressed', 'true')
    expect(requests.filter((url) => url === '/audit/events/stats')).toHaveLength(1)
    expect(requestedDays).toEqual([14])

    view.unmount()
    renderDashboard()
    expect(await screen.findByRole('button', { name: 'Area' })).toHaveAttribute('aria-pressed', 'true')
  })

  it('treats an all-zero response as successful chart data', async () => {
    setSession()
    useMenuStore.getState().setAuthorizedMenus([], true)
    http.defaults.adapter = successAdapter({
      ...auditStats,
      eventCount: 0,
      todayLogins: 0,
      todayIps: 0,
      daily: auditStats.daily.map((row) => ({
        ...row,
        logins: 0,
        ips: 0,
        loginFailures: 0,
        accessDenials: 0,
      })),
    })

    renderDashboard()

    expect(await screen.findByText('Login trend')).toBeInTheDocument()
    expect(screen.getByText('Security event trend')).toBeInTheDocument()
    expect(screen.getByLabelText('Login trend dates: 07-24, 07-25')).toBeInTheDocument()
    expect(within(screen.getByLabelText('Logins today')).getByText('0')).toBeInTheDocument()
    expect(within(screen.getByLabelText('IPs today')).getByText('0')).toBeInTheDocument()
    expect(screen.queryByText('No audit events')).not.toBeInTheDocument()
  })

  it('does not request or render audit statistics without audit-events permission', async () => {
    const requests: string[] = []
    setSession(false)
    useMenuStore.getState().setAuthorizedMenus([{ name: 'users', menuType: 'page' }])
    http.defaults.adapter = successAdapter(auditStats, requests)

    renderDashboard()

    const welcome = screen.getByRole('heading', { name: 'Welcome back, Admin.' })
    const usersLink = await screen.findByRole('link', { name: 'Users: 12' })
    expect(welcome.compareDocumentPosition(usersLink) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy()
    expect(screen.queryByLabelText('Logins today')).not.toBeInTheDocument()
    expect(screen.queryByText('Login trend')).not.toBeInTheDocument()
    expect(screen.queryByText('Security event trend')).not.toBeInTheDocument()
    expect(requests).not.toContain('/audit/events/stats')
  })

  it('shows loading placeholders without drawing zero charts', async () => {
    setSession()
    useMenuStore.getState().setAuthorizedMenus([], true)
    http.defaults.adapter = (async (config) => {
      const url = config.url ?? ''
      if (url === '/audit/events/stats') return await new Promise(() => {})
      const response = RESOURCE_RESPONSES[url]
      if (response === undefined) throw new Error(`Unexpected request: ${config.method} ${url}`)
      return {
        data: { code: 'OK', message: 'ok', data: response },
        status: 200,
        statusText: 'OK',
        headers: {},
        config,
      }
    }) as AxiosAdapter

    renderDashboard()

    expect(within(screen.getByLabelText('Logins today')).getByText('…')).toBeInTheDocument()
    expect(within(screen.getByLabelText('IPs today')).getByText('…')).toBeInTheDocument()
    expect(screen.getAllByText('Loading statistics…')).toHaveLength(2)
    expect(screen.queryByText('Last 14 days · UTC')).not.toBeInTheDocument()
  })

  it('shows one shared error block and dashes when audit statistics fail', async () => {
    setSession()
    useMenuStore.getState().setAuthorizedMenus([], true)
    http.defaults.adapter = (async (config) => {
      const url = config.url ?? ''
      if (url === '/audit/events/stats') throw new Error('stats failed')
      const response = RESOURCE_RESPONSES[url]
      if (response === undefined) throw new Error(`Unexpected request: ${config.method} ${url}`)
      return {
        data: { code: 'OK', message: 'ok', data: response },
        status: 200,
        statusText: 'OK',
        headers: {},
        config,
      }
    }) as AxiosAdapter

    renderDashboard()

    expect(await screen.findByText('Failed to load statistics')).toBeInTheDocument()
    expect(screen.getAllByText('Failed to load statistics')).toHaveLength(1)
    expect(within(screen.getByLabelText('Logins today')).getByText('—')).toBeInTheDocument()
    expect(within(screen.getByLabelText('IPs today')).getByText('—')).toBeInTheDocument()
    await waitFor(() => expect(screen.queryByText('Login trend')).not.toBeInTheDocument())
  })
})
