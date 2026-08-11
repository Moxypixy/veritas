import { describe, expect, it } from 'vitest'

import inflationConfig from '../../crates/veritas-inflation/regions.json'
import { REGIONS } from './regions'

type Region = { id: string; label: string }

describe('shared regions', () => {
  it('exposes more than the original four OECD areas', () => {
    expect(REGIONS.length).toBeGreaterThan(4)
    expect(REGIONS.every((region) => region.id && region.label)).toBe(true)
  })

  it('matches the inflation region configuration in order', () => {
    const inflationRegions = (inflationConfig as { regions: Region[] }).regions
      .map(({ id, label }) => ({ id, label }))

    expect(REGIONS).toEqual(inflationRegions)
  })
})
