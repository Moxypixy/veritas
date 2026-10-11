import { schnorr } from '@noble/curves/secp256k1.js'
import { describe, expect, it } from 'vitest'

import { attestationMessage, bytesToHex, foldBatchRoots, kip5Digest, leafNode, recordLeaf, verify } from '../veritas/verify'
import { composePlace, draftToRecord, toMinorUnits, toQtyMilli, type Draft } from './AttestPage'

const draft: Draft = {
  item: 'milk.whole.1l.dk',
  amount: '12,95',
  currency: 'dkk',
  quantity: '1',
  unit: 'l',
  area: 'København',
  shop: 'Netto Nørrebrogade',
  includeShop: false,
  observedOn: '2026-10-10',
  source: 'live',
}

describe('AttestPage', () => {
  it('converts what people type into integer minor units and thousandths', () => {
    expect(toMinorUnits('12.95', 'DKK')).toBe(1295)
    expect(toMinorUnits('12,9', 'DKK')).toBe(1290)
    expect(toMinorUnits('250', 'JPY')).toBe(250)
    expect(() => toMinorUnits('2.5', 'JPY')).toThrow(/0 decimals/)
    expect(toQtyMilli('0.5')).toBe(500)
    expect(toQtyMilli('1,75')).toBe(1750)
    expect(() => toQtyMilli('1.2345')).toThrow()
  })

  it('builds a record whose KIP-5 signature the verifier accepts', () => {
    const secret = new Uint8Array(32).fill(9)
    const attester = bytesToHex(schnorr.getPublicKey(secret))
    const record = draftToRecord(draft, attester, new Uint8Array(16).fill(1), null)
    expect(record.currency).toBe('DKK')
    expect(record.amount_minor).toBe(1295)

    const signature = schnorr.sign(kip5Digest(attestationMessage(record, recordLeaf(record))), secret)
    const signed = { record, signature: bytesToHex(signature) }
    // A one-record batch: the leaf alone is the root; the log holds that root.
    const root = leafNode(recordLeaf(record))
    const result = verify(
      signed,
      { version: 1, batch_index: 0, batch_root: bytesToHex(root), leaf_index: 0, proof: [] },
      { batch_roots: [bytesToHex(root)] },
      bytesToHex(foldBatchRoots([root])),
    )
    expect(result.ok).toBe(true)
  })

  it('rejects an invalid item before anything is signed', () => {
    expect(() => draftToRecord({ ...draft, item: 'Milk' }, '00'.repeat(32), new Uint8Array(16), null)).toThrow(/item/)
  })

  it('stores only the area unless the attester opts in to the shop', () => {
    expect(draftToRecord(draft, '00'.repeat(32), new Uint8Array(16), null).place).toBe('København')
    expect(composePlace({ ...draft, includeShop: true })).toBe('København · Netto Nørrebrogade')
    expect(composePlace({ area: '', shop: 'Netto', includeShop: true })).toBe('Netto')
    expect(composePlace({ area: '  ', shop: '', includeShop: true })).toBe('')
  })
})
