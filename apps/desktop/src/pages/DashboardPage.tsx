import { useState } from 'react'
import { useQueries, useQuery } from '@tanstack/react-query'
import {
  IconActivity,
  IconBookmarks,
  IconBuilding,
  IconLogin2,
  IconNetwork,
  IconSettings,
  IconShield,
  IconStack2,
  IconUsers,
  type Icon,
} from '@tabler/icons-react'
import { useTranslation } from 'react-i18next'
import { Link } from 'react-router-dom'
import { Area, AreaChart, Bar, BarChart, CartesianGrid, ResponsiveContainer, Tooltip, XAxis, YAxis } from 'recharts'

import { fetchAuditStats } from '@/api/audit'
import { listDepartments, type DeptRecord } from '@/api/departments'
import { fetchDictionaries } from '@/api/dictionaries'
import { fetchFiles } from '@/api/files'
import { fetchParams } from '@/api/params'
import { listRoles } from '@/api/roles'
import { fetchUsers } from '@/api/users'
import { PageHeader } from '@/components/PageHeader'
import { Button } from '@/components/ui/Button'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { useAuthStore } from '@/stores/auth'
import { useMenuStore } from '@/stores/menu'

import { createLoginSeries, securitySeries, type LoginChartMode } from './dashboardChartConfig'

const chartTooltip = {
  background: 'var(--popover)',
  border: '1px solid var(--border)',
  borderRadius: 8,
  fontSize: 12,
}
const chartTickStyle = { fill: 'var(--muted-foreground)', fontSize: 11 }

const RESOURCE_CARDS: Array<{
  key: string
  label: string
  path: string
  icon: Icon
}> = [
  { key: 'users', label: 'Users', path: '/users', icon: IconUsers },
  { key: 'roles', label: 'Roles', path: '/roles', icon: IconShield },
  { key: 'departments', label: 'Departments', path: '/departments', icon: IconBuilding },
  { key: 'files', label: 'Files', path: '/files', icon: IconStack2 },
  { key: 'params', label: 'Params', path: '/params', icon: IconSettings },
  { key: 'dictionaries', label: 'Dictionaries', path: '/dictionaries', icon: IconBookmarks },
  { key: 'audit-events', label: 'Audit events', path: '/audit-events', icon: IconActivity },
]

function countDepartments(nodes: DeptRecord[]): number {
  return nodes.reduce((sum, node) => sum + 1 + countDepartments(node.children ?? []), 0)
}

function MetricCard({ icon: Icon, label, value }: { icon: Icon; label: string; value: string }) {
  return (
    <section aria-label={label}>
      <Card className="h-full">
        <CardContent className="flex items-center gap-3 py-2">
          <div className="flex size-10 shrink-0 items-center justify-center rounded-lg bg-primary/10 text-primary">
            <Icon className="size-5" />
          </div>
          <div className="min-w-0">
            <p className="truncate text-sm text-muted-foreground">{label}</p>
            <strong className="text-2xl font-semibold">{value}</strong>
          </div>
        </CardContent>
      </Card>
    </section>
  )
}

function ChartLegend({ label, items }: { label: string; items: Array<{ color: string; label: string }> }) {
  return (
    <div aria-label={label} className="mb-3 flex flex-wrap gap-x-4 gap-y-1 text-xs text-muted-foreground" role="group">
      {items.map((item) => (
        <span className="inline-flex items-center gap-1.5" key={item.label}>
          <span aria-hidden className="size-2 rounded-full" style={{ backgroundColor: item.color }} />
          {item.label}
        </span>
      ))}
    </div>
  )
}

export function DashboardPage() {
  const { t } = useTranslation()
  const [loginChartMode, setLoginChartMode] = useState<LoginChartMode>('area')
  const user = useAuthStore((state) => state.userInfo)
  const canAccess = useMenuStore((state) => state.canAccess)

  const visibleCards = RESOURCE_CARDS.filter((card) => canAccess(card.key))
  const canAudit = canAccess('audit-events')

  const resourceQueries = useQueries({
    queries: [
      {
        queryKey: ['dashboard', 'users'],
        queryFn: () => fetchUsers({ page: 1, pageSize: 1 }),
        enabled: canAccess('users'),
      },
      {
        queryKey: ['dashboard', 'roles'],
        queryFn: listRoles,
        enabled: canAccess('roles'),
      },
      {
        queryKey: ['dashboard', 'departments'],
        queryFn: listDepartments,
        enabled: canAccess('departments'),
      },
      {
        queryKey: ['dashboard', 'files'],
        queryFn: () => fetchFiles({ page: 1, pageSize: 1 }),
        enabled: canAccess('files'),
      },
      {
        queryKey: ['dashboard', 'params'],
        queryFn: () => fetchParams({ page: 1, pageSize: 1 }),
        enabled: canAccess('params'),
      },
      {
        queryKey: ['dashboard', 'dictionaries'],
        queryFn: () => fetchDictionaries(),
        enabled: canAccess('dictionaries'),
      },
    ],
  })

  const [usersQ, rolesQ, deptsQ, filesQ, paramsQ, dictsQ] = resourceQueries
  const stats = useQuery({
    queryKey: ['dashboard', 'audit-stats', 14],
    queryFn: () => fetchAuditStats(14),
    enabled: canAudit,
  })

  const totals: Record<string, number | undefined> = {
    users: usersQ.data?.total,
    roles: rolesQ.data?.length,
    departments: deptsQ.data ? countDepartments(deptsQ.data) : undefined,
    files: filesQ.data?.total,
    params: paramsQ.data?.total,
    dictionaries: dictsQ.data?.length,
    'audit-events': stats.data?.eventCount,
  }
  const loadingByKey: Record<string, boolean> = {
    users: usersQ.isLoading,
    roles: rolesQ.isLoading,
    departments: deptsQ.isLoading,
    files: filesQ.isLoading,
    params: paramsQ.isLoading,
    dictionaries: dictsQ.isLoading,
    'audit-events': stats.isLoading,
  }
  const metricValue = (value: number | undefined) =>
    stats.isLoading ? '…' : stats.isError || value === undefined ? '—' : value.toLocaleString()
  const dailyData =
    stats.data?.daily.map((row) => ({
      label: row.date.slice(5),
      logins: row.logins,
      ips: row.ips,
      loginFailures: row.loginFailures,
      accessDenials: row.accessDenials,
    })) ?? []
  const chartWindowLabel = stats.data ? t('Last {{days}} days · UTC', { days: stats.data.days }) : ''
  const loginSeries = createLoginSeries(loginChartMode)

  return (
    <div className="space-y-5 xl:space-y-6">
      <PageHeader
        description={
          <h1 className="text-lg font-semibold text-foreground xl:text-xl">
            {t('Welcome back')}, {user?.nickName || user?.userName}.
          </h1>
        }
      />

      {canAudit && (
        <>
          <div className="grid grid-cols-1 gap-3 sm:grid-cols-2 xl:gap-4">
            <MetricCard icon={IconLogin2} label={t('Logins today')} value={metricValue(stats.data?.todayLogins)} />
            <MetricCard icon={IconNetwork} label={t('IPs today')} value={metricValue(stats.data?.todayIps)} />
          </div>

          {stats.isLoading ? (
            <div className="grid grid-cols-1 gap-4 lg:grid-cols-2 xl:gap-5">
              {[0, 1].map((slot) => (
                <Card key={slot}>
                  <CardContent className="flex min-h-[300px] items-center justify-center text-sm text-muted-foreground">
                    {t('Loading statistics…')}
                  </CardContent>
                </Card>
              ))}
            </div>
          ) : stats.isError ? (
            <Card>
              <CardContent className="flex min-h-32 items-center justify-center text-sm text-muted-foreground">
                {t('Failed to load statistics')}
              </CardContent>
            </Card>
          ) : stats.data ? (
            <div className="grid grid-cols-1 gap-4 lg:grid-cols-2 xl:gap-5">
              <Card>
                <CardHeader className="flex-row items-start justify-between gap-3">
                  <div>
                    <CardTitle>{t('Login trend')}</CardTitle>
                    <p className="mt-1 text-xs text-muted-foreground">{chartWindowLabel}</p>
                  </div>
                  <div aria-label={t('Login chart display')} className="flex gap-1" role="group">
                    <Button
                      aria-pressed={loginChartMode === 'area'}
                      onClick={() => setLoginChartMode('area')}
                      size="xs"
                      variant={loginChartMode === 'area' ? 'default' : 'outline'}
                    >
                      {t('Area')}
                    </Button>
                    <Button
                      aria-pressed={loginChartMode === 'line'}
                      onClick={() => setLoginChartMode('line')}
                      size="xs"
                      variant={loginChartMode === 'line' ? 'default' : 'outline'}
                    >
                      {t('Line')}
                    </Button>
                  </div>
                </CardHeader>
                <CardContent>
                  <ChartLegend
                    items={loginSeries.map((series) => ({
                      color: series.color,
                      label: t(series.labelKey),
                    }))}
                    label={t('Login trend series')}
                  />
                  <div
                    aria-label={t('Login trend dates: {{dates}}', {
                      dates: dailyData.map((row) => row.label).join(', '),
                    })}
                  >
                    <ResponsiveContainer className="min-h-[240px] xl:min-h-[280px]" height={240} width="100%">
                      <AreaChart data={dailyData}>
                        <defs>
                          {loginSeries.map((series) => (
                            <linearGradient id={series.gradient.id} key={series.dataKey} x1="0" x2="0" y1="0" y2="1">
                              <stop offset="0%" stopColor={series.color} stopOpacity={series.gradient.startOpacity} />
                              <stop offset="100%" stopColor={series.color} stopOpacity={series.gradient.endOpacity} />
                            </linearGradient>
                          ))}
                        </defs>
                        <CartesianGrid stroke="var(--border)" strokeDasharray="3 6" vertical={false} />
                        <XAxis axisLine={false} dataKey="label" tick={chartTickStyle} tickLine={false} />
                        <YAxis
                          allowDecimals={false}
                          axisLine={false}
                          tick={chartTickStyle}
                          tickLine={false}
                          width={28}
                        />
                        <Tooltip contentStyle={chartTooltip} />
                        {loginSeries.map((series) => (
                          <Area
                            dataKey={series.dataKey}
                            fill={series.fill}
                            fillOpacity={series.fillOpacity}
                            isAnimationActive={false}
                            key={series.dataKey}
                            name={t(series.labelKey)}
                            stroke={series.color}
                            strokeWidth={2}
                            type="monotone"
                          />
                        ))}
                      </AreaChart>
                    </ResponsiveContainer>
                  </div>
                </CardContent>
              </Card>

              <Card>
                <CardHeader>
                  <CardTitle>{t('Security event trend')}</CardTitle>
                  <p className="text-xs text-muted-foreground">{chartWindowLabel}</p>
                </CardHeader>
                <CardContent>
                  <ChartLegend
                    items={securitySeries.map((series) => ({
                      color: series.color,
                      label: t(series.labelKey),
                    }))}
                    label={t('Security event trend series')}
                  />
                  <ResponsiveContainer className="min-h-[240px] xl:min-h-[280px]" height={240} width="100%">
                    <BarChart data={dailyData}>
                      <CartesianGrid stroke="var(--border)" strokeDasharray="3 6" vertical={false} />
                      <XAxis axisLine={false} dataKey="label" tick={chartTickStyle} tickLine={false} />
                      <YAxis allowDecimals={false} axisLine={false} tick={chartTickStyle} tickLine={false} width={28} />
                      <Tooltip contentStyle={chartTooltip} />
                      {securitySeries.map((series, index) => (
                        <Bar
                          dataKey={series.dataKey}
                          fill={series.color}
                          isAnimationActive={false}
                          key={series.dataKey}
                          name={t(series.labelKey)}
                          radius={index === securitySeries.length - 1 ? [4, 4, 0, 0] : undefined}
                          stackId={series.stackId}
                        />
                      ))}
                    </BarChart>
                  </ResponsiveContainer>
                </CardContent>
              </Card>
            </div>
          ) : null}
        </>
      )}

      {visibleCards.length > 0 && (
        <div className="grid grid-cols-2 gap-3 sm:grid-cols-3 md:grid-cols-4 xl:grid-cols-4 2xl:grid-cols-7 xl:gap-4">
          {visibleCards.map((card) => {
            const Icon = card.icon
            const value = totals[card.key]
            const loading = loadingByKey[card.key]
            const valueLabel = loading ? '…' : value == null ? '—' : value.toLocaleString()
            return (
              <Link aria-label={`${t(card.label)}: ${valueLabel}`} key={card.key} to={card.path}>
                <Card className="h-full transition-colors hover:border-primary/40 hover:bg-accent/40">
                  <CardContent className="flex items-center gap-3 py-1">
                    <div className="flex size-9 shrink-0 items-center justify-center rounded-lg bg-primary/10 text-primary xl:size-10">
                      <Icon className="size-4 xl:size-5" />
                    </div>
                    <div className="min-w-0">
                      <p className="truncate text-xs text-muted-foreground xl:text-sm">{t(card.label)}</p>
                      <strong className="text-lg font-semibold xl:text-xl">{valueLabel}</strong>
                    </div>
                  </CardContent>
                </Card>
              </Link>
            )
          })}
        </div>
      )}
    </div>
  )
}
