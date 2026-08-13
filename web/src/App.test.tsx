import { renderToStaticMarkup } from 'react-dom/server'
import { afterEach, describe, expect, it } from 'vitest'

import App from './App'

const originalWindow = globalThis.window

afterEach(() => {
  Object.defineProperty(globalThis, 'window', {
    configurable: true,
    value: originalWindow,
  })
})

describe('App', () => {
  it('places the chart and its region control before the vote form', () => {
    Object.defineProperty(globalThis, 'window', {
      configurable: true,
      value: { location: { pathname: '/' } },
    })

    const page = renderToStaticMarkup(<App />)

    expect(page.indexOf('Cost-of-living chart')).toBeGreaterThan(-1)
    expect(page.indexOf('Region')).toBeLessThan(page.indexOf('About your household costs'))
  })
})
