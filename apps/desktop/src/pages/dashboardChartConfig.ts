export type LoginChartMode = 'area' | 'line'

interface LoginSeriesDefinition {
  dataKey: 'logins' | 'ips'
  labelKey: 'Logins' | 'IPs'
  color: string
  gradient: {
    id: string
    startOpacity: number
    endOpacity: number
  }
}

interface LoginSeries extends LoginSeriesDefinition {
  fill: string
  fillOpacity: number
}

const loginSeriesDefinitions: LoginSeriesDefinition[] = [
  {
    dataKey: 'logins',
    labelKey: 'Logins',
    color: 'var(--chart-1)',
    gradient: { id: 'dashboardLoginsFill', startOpacity: 0.28, endOpacity: 0.02 },
  },
  {
    dataKey: 'ips',
    labelKey: 'IPs',
    color: 'var(--chart-2)',
    gradient: { id: 'dashboardIpsFill', startOpacity: 0.2, endOpacity: 0.02 },
  },
]

export function createLoginSeries(mode: LoginChartMode): LoginSeries[] {
  const showArea = mode === 'area'
  return loginSeriesDefinitions.map((series) => ({
    ...series,
    fill: showArea ? `url(#${series.gradient.id})` : 'transparent',
    fillOpacity: showArea ? 1 : 0,
  }))
}

export const securitySeries = [
  {
    dataKey: 'loginFailures',
    labelKey: 'Login failures',
    color: 'var(--chart-1)',
    stackId: 'security',
  },
  {
    dataKey: 'accessDenials',
    labelKey: 'Access denials',
    color: 'var(--chart-2)',
    stackId: 'security',
  },
] as const
