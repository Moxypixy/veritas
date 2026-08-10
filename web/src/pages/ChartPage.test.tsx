import { renderToStaticMarkup } from 'react-dom/server'
import { describe, expect, it } from 'vitest'

import {
  ChartContent,
  ChartDataSection,
  chartRequestUrl,
  type ChartResponse,
} from './ChartPage'

const allResponse: ChartResponse = {
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
      points: [{ month_key: '2026-08', value: 55 }],
    },
  ],
}

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

  it('shows loading instead of prior chart values when response is cleared', () => {
    const chart = renderToStaticMarkup(<ChartDataSection error={null} response={null} />)

    expect(chart).toContain('Loading chart data')
    expect(chart).not.toContain('2.1%')
    expect(chart).not.toContain('Monthly percentages')
  })

  it('shows chart content only when a response is available', () => {
    const loading = renderToStaticMarkup(<ChartDataSection error={null} response={null} />)
    const loaded = renderToStaticMarkup(<ChartDataSection error={null} response={allResponse} />)

    expect(loading).toContain('Loading chart data')
    expect(loaded).toContain('2.1%')
    expect(loaded).not.toContain('Loading chart data')
  })
})
