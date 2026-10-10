import { describe, expect, it } from 'vitest'

import log from '../../../examples/demo/anchor-log.json'
import march from '../../../examples/demo/milk-2026-03.signed.json'
import october from '../../../examples/demo/milk-2026-10.signed.json'
import usGallon from '../../../examples/demo/milk-us-gallon.signed.json'
import marchReceipt from '../../../examples/demo/receipts/milk-2026-03.receipt.json'
import octoberReceipt from '../../../examples/demo/receipts/milk-2026-10.receipt.json'
import usGallonReceipt from '../../../examples/demo/receipts/milk-us-gallon.receipt.json'
import liveAccFile from '../../../examples/demo/live-acc.txt?raw'

import {
  bytesToHex,
  foldBatchRoots,
  hexToBytes,
  pricePerBaseMilli,
  verify,
  type AnchorLog,
  type Receipt,
  type SignedRecord,
} from './verify'

const liveAcc = liveAccFile.trim()
const anchorLog = log as AnchorLog
const fixtures: [string, SignedRecord, Receipt][] = [
  ['March milk', march as SignedRecord, marchReceipt as Receipt],
  ['October milk', october as SignedRecord, octoberReceipt as Receipt],
  ['US gallon', usGallon as SignedRecord, usGallonReceipt as Receipt],
]

describe('Veritas browser verifier', () => {
  it('folds the demo log to the live accumulator written by the Rust CLI', () => {
    expect(bytesToHex(foldBatchRoots(anchorLog.batch_roots.map((r) => hexToBytes(r, 32))))).toBe(liveAcc)
  })

  it.each(fixtures)('says YES to %s', (_name, signed, receipt) => {
    expect(verify(signed, receipt, anchorLog, liveAcc).ok).toBe(true)
  })

  it('reports the same unit prices as Rust', () => {
    expect(pricePerBaseMilli(431, 1000, 'us_gal')).toBe(113_858n)
    const result = verify(march as SignedRecord, marchReceipt as Receipt, anchorLog, liveAcc)
    expect(result.ok && result.pricePerBaseMilli).toBe(1_195_000n)
  })

  it('says NO when one digit changes', () => {
    const edited = structuredClone(march) as SignedRecord
    edited.record.amount_minor = 995
    const result = verify(edited, marchReceipt as Receipt, anchorLog, liveAcc)
    expect(result).toEqual({ ok: false, reason: 'Signature does not match the record and attester.' })
  })

  it('says NO when the log has been rewritten', () => {
    const rewritten = { batch_roots: [...anchorLog.batch_roots] }
    rewritten.batch_roots[1] = 'ee'.repeat(32)
    const result = verify(march as SignedRecord, marchReceipt as Receipt, rewritten, liveAcc)
    expect(result).toEqual({ ok: false, reason: 'Anchor log does not hash to the live covenant root.' })
  })

  it('says NO for a receipt naming the wrong batch', () => {
    const wrong = { ...(marchReceipt as Receipt), batch_index: 1 }
    expect(verify(march as SignedRecord, wrong, anchorLog, liveAcc).ok).toBe(false)
  })

  it('rejects ambiguous units and malformed input without throwing', () => {
    const pint = structuredClone(march) as SignedRecord
    ;(pint.record as { unit: string }).unit = 'pt'
    expect(verify(pint, marchReceipt as Receipt, anchorLog, liveAcc)).toEqual({ ok: false, reason: 'Record is malformed: unknown unit "pt"' })
    expect(verify(march as SignedRecord, marchReceipt as Receipt, anchorLog, 'not-hex').ok).toBe(false)
  })
})
