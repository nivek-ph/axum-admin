export type LoginChartMode = 'area' | 'line'

interface LoginSeries {
  dataKey: 'logins' | 'ips'
  color: string
  fill: string
  fillOpacity: number
}

export function createLoginSeries(mode: LoginChartMode): LoginSeries[] {
  const showArea = mode === 'area'
  return [
    {
      dataKey: 'logins',
      color: 'var(--chart-1)',
      fill: showArea ? 'url(#dashboardLoginsFill)' : 'transparent',
      fillOpacity: showArea ? 1 : 0,
    },
    {
      dataKey: 'ips',
      color: 'var(--chart-2)',
      fill: showArea ? 'url(#dashboardIpsFill)' : 'transparent',
      fillOpacity: showArea ? 1 : 0,
    },
  ]
}

export const securitySeries = [
  {
    dataKey: 'loginFailures',
    color: 'var(--chart-1)',
    stackId: 'security',
  },
  {
    dataKey: 'accessDenials',
    color: 'var(--chart-2)',
    stackId: 'security',
  },
] as const
