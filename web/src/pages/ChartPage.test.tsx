import { renderToStaticMarkup } from 'react-dom/server'
import { describe, expect, it } from 'vitest'

import { ChartContent, chartRequestUrl, type ChartResponse } from './ChartPage'

const suppressedResponse: ChartResponse = {
  region_id: 'euro-area',
  official_unavailable: false,
  series: [
    {
      key: 'prices_overall',
      label: 'How fast prices are rising overall',
      points: [{ month_key: '2026-08', value: 2.1 }],
    },
    {
      key: 'voters_necessities',
      label: 'What voters say they spend on necessities',
      points: [],
    },
  ],
}

describe('ChartPage', () => {
  it('explains suppressed voter aggregates without exposing an individual response', async () => {
    const chart = renderToStaticMarkup(<ChartContent response={suppressedResponse} />)

    expect(chart).toContain('Not enough responses yet')
    expect(chart).toContain('How fast prices are rising overall')
    expect(chart).not.toContain('wallet')
  })

  it('uses the selected employment filter when requesting chart data', () => {
    expect(chartRequestUrl('euro-area', 'unemployed')).toBe(
      '/v1/chart?region_id=euro-area&employment=unemployed',
    )
  })
})
