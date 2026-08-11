import { describe, expect, it } from 'vitest'

import { canonicalBytes, commitmentHash } from './commitment'

const goldenAnswer = {
  regionId: 'euro-area',
  necessitiesPct: 42,
  employed: true,
  durationMonths: 18,
  monthKey: '2026-08',
  wallet: 'kaspatest:qqveritasgoldenwallet1',
  salt: new Uint8Array(16).fill(7),
}

describe('commitment encoding', () => {
  it('matches Rust golden vector 1 byte-for-byte and by BLAKE2b-256 digest', () => {
    expect([...canonicalBytes(goldenAnswer)]).toEqual([
      15, 0, 0, 0, ...new TextEncoder().encode('veritas.colp.v1'),
      9, 0, 0, 0, ...new TextEncoder().encode('euro-area'),
      42, 1,
      18, 0, 0, 0,
      7, 0, 0, 0, ...new TextEncoder().encode('2026-08'),
      32, 0, 0, 0, ...new TextEncoder().encode('kaspatest:qqveritasgoldenwallet1'),
      ...new Uint8Array(16).fill(7),
    ])
    expect(commitmentHash(goldenAnswer)).toBe(
      'ebbca110bc4f0c17b38ea1b8e5a447e37a77ebc80a9192003fc10bb5e97c1dcb',
    )
  })

  it('matches Rust golden vector 2 when employment is false', () => {
    expect(commitmentHash({ ...goldenAnswer, employed: false })).toBe(
      'a7d78af0d17c3abeb78582cd812304d146305821943bb24788cd9e42c7600a0d',
    )
  })

  it('rejects invalid canonical values', () => {
    expect(() => canonicalBytes({ ...goldenAnswer, necessitiesPct: 101 })).toThrow(
      'necessities percentage must be between 0 and 100',
    )
    expect(() => canonicalBytes({ ...goldenAnswer, regionId: 'europé' })).toThrow(
      'region ID must contain only ASCII',
    )
    expect(() => canonicalBytes({ ...goldenAnswer, monthKey: '2026-8' })).toThrow(
      'month key must match YYYY-MM',
    )
    expect(() => canonicalBytes({ ...goldenAnswer, wallet: '' })).toThrow(
      'wallet must not be empty',
    )
  })
})
