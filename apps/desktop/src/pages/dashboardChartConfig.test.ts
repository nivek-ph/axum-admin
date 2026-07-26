import { describe, expect, it } from 'vitest'

import { createLoginSeries, securitySeries } from './dashboardChartConfig'

describe('Dashboard chart configuration', () => {
  it('uses filled areas by default and removes both fills in line mode', () => {
    expect(createLoginSeries('area').map((series) => series.fillOpacity)).toEqual([1, 1])
    expect(createLoginSeries('line')).toEqual([
      expect.objectContaining({ dataKey: 'logins', fill: 'transparent', fillOpacity: 0 }),
      expect.objectContaining({ dataKey: 'ips', fill: 'transparent', fillOpacity: 0 }),
    ])
  })

  it('stacks both security event series together', () => {
    expect(securitySeries.map((series) => series.dataKey)).toEqual(['loginFailures', 'accessDenials'])
    expect(new Set(securitySeries.map((series) => series.stackId))).toEqual(new Set(['security']))
  })
})
