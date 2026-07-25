export type LoginChartMode = 'area' | 'line'

interface LoginSeries {
  dataKey: 'successfulLogins' | 'uniqueIps'
  color: string
  fill: string
  fillOpacity: number
}

export function createLoginSeries(mode: LoginChartMode): LoginSeries[] {
  const showArea = mode === 'area'
  return [
    {
      dataKey: 'successfulLogins',
      color: 'var(--chart-1)',
      fill: showArea ? 'url(#dashboardSuccessfulLoginsFill)' : 'transparent',
      fillOpacity: showArea ? 1 : 0,
    },
    {
      dataKey: 'uniqueIps',
      color: 'var(--chart-2)',
      fill: showArea ? 'url(#dashboardUniqueIpsFill)' : 'transparent',
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
