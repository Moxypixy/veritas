import { useEffect, useState } from 'react'

const REGIONS = [
  { id: 'euro-area', label: 'Euro area' },
  { id: 'united-states', label: 'United States' },
  { id: 'united-kingdom', label: 'United Kingdom' },
  { id: 'japan', label: 'Japan' },
] as const

const EMPLOYMENT_FILTERS = [
  { value: 'all', label: 'All' },
  { value: 'employed', label: 'Employed' },
  { value: 'unemployed', label: 'Unemployed' },
] as const

const SERIES_NAMES: Record<string, string> = {
  prices_overall: 'How fast prices are rising overall',
  prices_food: 'Food prices',
  prices_housing: 'Housing costs',
  prices_energy: 'Energy and fuel prices',
  voters_necessities: 'What voters say they spend on necessities',
}

type EmploymentFilter = typeof EMPLOYMENT_FILTERS[number]['value']

export type ChartPoint = {
  month_key: string
  value: number
}

export type ChartSeries = {
  key: string
  label: string
  points: ChartPoint[]
}

export type ChartResponse = {
  region_id: string
  official_unavailable: boolean
  series: ChartSeries[]
}

type ChartPageProps = {
  loadChart?: (url: string) => Promise<ChartResponse>
}

function loadChart(url: string): Promise<ChartResponse> {
  return fetch(url).then(async (response) => {
    if (!response.ok) {
      throw new Error('The chart data could not be loaded.')
    }
    return response.json() as Promise<ChartResponse>
  })
}

export function chartRequestUrl(regionId: string, employment: EmploymentFilter) {
  return `/v1/chart?region_id=${encodeURIComponent(regionId)}&employment=${employment}`
}

function dateLabel(monthKey: string) {
  const [year, month] = monthKey.split('-')
  if (!year || !month) return monthKey
  return new Intl.DateTimeFormat('en', { month: 'long', year: 'numeric', timeZone: 'UTC' })
    .format(new Date(Date.UTC(Number(year), Number(month) - 1)))
}

function displaySeries(series: ChartSeries[]) {
  return series.filter((item) => Object.hasOwn(SERIES_NAMES, item.key))
}

function latestMonth(series: ChartSeries[]) {
  return series.flatMap((item) => item.points.map((point) => point.month_key))
    .sort()
    .at(-1)
}

export function ChartContent({ response }: { response: ChartResponse }) {
  const series = displaySeries(response.series)
  const voterSeries = series.find((item) => item.key === 'voters_necessities')
  const hasVoterAggregate = voterSeries?.points.length !== 0
  const months = [...new Set(series.flatMap((item) => item.points.map((point) => point.month_key)))].sort()
  const updated = latestMonth(series)

  return (
    <>
      {response.official_unavailable && (
        <p className="status" role="status">Official price data is temporarily unavailable. Voter averages remain available when enough people have responded.</p>
      )}
      {!hasVoterAggregate && (
        <p className="status" role="status">Not enough responses yet. We only show voter averages when at least five responses protect people’s privacy.</p>
      )}
      {months.length === 0 ? (
        <p className="status" role="status">No chart data is available for this region yet.</p>
      ) : (
        <section aria-labelledby="chart-data-heading">
          <h2 id="chart-data-heading">Monthly percentages</h2>
          <p id="chart-description">Each row is a month. Values are percentages, and an em dash means that series has no result for that month.</p>
          <div className="chart-table-wrap">
            <table aria-describedby="chart-description">
              <caption>Cost-of-living data by month</caption>
              <thead>
                <tr>
                  <th scope="col">Month</th>
                  {series.map((item) => <th key={item.key} scope="col">{SERIES_NAMES[item.key]}</th>)}
                </tr>
              </thead>
              <tbody>
                {months.map((month) => (
                  <tr key={month}>
                    <th scope="row">{dateLabel(month)}</th>
                    {series.map((item) => {
                      const point = item.points.find((candidate) => candidate.month_key === month)
                      return <td key={item.key}>{point ? `${point.value.toFixed(1)}%` : '—'}</td>
                    })}
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
          <footer className="chart-footnotes">
            <p><strong>Dataset:</strong> Official public price statistics and anonymous Cost-of-Living Pulse responses.</p>
            {updated && <p><strong>Last updated:</strong> Latest available chart month: {dateLabel(updated)}.</p>}
          </footer>
        </section>
      )}
    </>
  )
}

export function ChartDataSection({
  error,
  response,
}: {
  error: string | null
  response: ChartResponse | null
}) {
  if (error) {
    return <p className="status" role="alert">{error}</p>
  }
  if (!response) {
    return <p className="status" role="status">Loading chart data…</p>
  }
  return <ChartContent response={response} />
}

export default function ChartPage({ loadChart: requestChart = loadChart }: ChartPageProps) {
  const [regionId, setRegionId] = useState('euro-area')
  const [employment, setEmployment] = useState<EmploymentFilter>('all')
  const [response, setResponse] = useState<ChartResponse | null>(null)
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    let active = true
    setError(null)
    setResponse(null)
    requestChart(chartRequestUrl(regionId, employment))
      .then((data) => {
        if (active) setResponse(data)
      })
      .catch(() => {
        if (active) setError('The chart data could not be loaded. Please try again later.')
      })
    return () => { active = false }
  }, [employment, regionId, requestChart])

  return (
    <main className="page">
      <nav className="site-nav" aria-label="Primary navigation">
        <a href="/">Vote</a>
        <a href="/chart" aria-current="page">Chart</a>
      </nav>
      <header className="hero">
        <p className="eyebrow">Cost-of-Living Pulse · Kaspa testnet</p>
        <h1>Cost-of-living chart</h1>
        <p>Compare official price changes with anonymous monthly averages reported by voters. No individual votes or wallet addresses are shown.</p>
      </header>

      <section className="card">
        <label htmlFor="chart-region">Region</label>
        <select id="chart-region" value={regionId} onChange={(event) => setRegionId(event.target.value)}>
          {REGIONS.map((region) => <option key={region.id} value={region.id}>{region.label}</option>)}
        </select>

        <fieldset className="chart-filters">
          <legend>Employment filter</legend>
          <div className="filter-buttons">
            {EMPLOYMENT_FILTERS.map((filter) => (
              <button
                aria-pressed={employment === filter.value}
                className="filter-button"
                key={filter.value}
                onClick={() => setEmployment(filter.value)}
                type="button"
              >
                {filter.label}
              </button>
            ))}
          </div>
        </fieldset>

        <ChartDataSection error={error} response={response} />
      </section>
    </main>
  )
}
