import { describe, expect, it } from 'vitest'

import { REGIONS } from './regions'

describe('shared regions', () => {
  it('exposes more than the original four OECD areas', () => {
    expect(REGIONS.length).toBeGreaterThan(4)
    expect(REGIONS.every((region) => region.id && region.label)).toBe(true)
  })
})
