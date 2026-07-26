import { describe, expect, it } from 'vitest'

import { createLoginSeries, securitySeries } from './dashboardChartConfig'

describe('Dashboard chart configuration', () => {
  it('keeps login chart labels, colors, and gradients in one series definition', () => {
    expect(createLoginSeries('area')).toEqual([
      expect.objectContaining({
        dataKey: 'logins',
        labelKey: 'Logins',
        color: 'var(--chart-1)',
        gradient: { id: 'dashboardLoginsFill', startOpacity: 0.28, endOpacity: 0.02 },
      }),
      expect.objectContaining({
        dataKey: 'ips',
        labelKey: 'IPs',
        color: 'var(--chart-2)',
        gradient: { id: 'dashboardIpsFill', startOpacity: 0.2, endOpacity: 0.02 },
      }),
    ])
  })

  it('uses filled areas by default and removes both fills in line mode', () => {
    const areaSeries = createLoginSeries('area')
    expect(areaSeries.map((series) => series.fillOpacity)).toEqual([1, 1])
    expect(areaSeries.map((series) => series.fill)).toEqual(
      areaSeries.map((series) => `url(#${series.gradient.id})`),
    )
    expect(createLoginSeries('line')).toEqual([
      expect.objectContaining({ dataKey: 'logins', fill: 'transparent', fillOpacity: 0 }),
      expect.objectContaining({ dataKey: 'ips', fill: 'transparent', fillOpacity: 0 }),
    ])
  })

  it('stacks both security event series together', () => {
    expect(securitySeries).toEqual([
      expect.objectContaining({
        dataKey: 'loginFailures',
        labelKey: 'Login failures',
        color: 'var(--chart-1)',
      }),
      expect.objectContaining({
        dataKey: 'accessDenials',
        labelKey: 'Access denials',
        color: 'var(--chart-2)',
      }),
    ])
    expect(new Set(securitySeries.map((series) => series.stackId))).toEqual(new Set(['security']))
  })
})
