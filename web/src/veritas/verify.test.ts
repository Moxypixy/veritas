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
  attestationMessage,
  bytesToHex,
  kccHash,
  recordLeaf,
  verifyKip5,
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

describe('KIP-5 and KCC-1 primitives', () => {
  it('reproduces the official KIP-5 test vectors', () => {
    const vectors: [string, string, string][] = [
      ['F9308A019258C31049344F85F89D5229B531C845836F99B08601F113BCE036F9', 'Hello Kaspa!', '40B9BB2BE0AE02607279EDA64015A8D86E3763279170340B8243F7CE5344D77AFF1191598BAF2FD26149CAC3B4B12C2C433261C00834DB6098CB172AA48EF522'],
      ['DFF1D77F2A671C5F36183726DB2341BE58FEAE1DA2DECED843240F7B502BA659', 'Hello Kaspa!', 'EB9E8A3C547EB91B6A7592644F328F0648BDD21ABA3CD44787D429D4D790AA8B962745691F3B472ED8D65F3B770ECB4F777BD17B1D309100919B53E0E206B4C6'],
      ['DFF1D77F2A671C5F36183726DB2341BE58FEAE1DA2DECED843240F7B502BA659', 'こんにちは世界', '810653D5F80206DB519672362ADD6C98DAD378844E5BA4D89A22C9F0C7092E8CECBA734FFF7922B656B4BE3F4B1F098899C95CB5C1023DCE3519208AFAFB59BC'],
    ]
    for (const [publicKey, message, signature] of vectors) {
      expect(verifyKip5(message, hexToBytes(signature, 64), hexToBytes(publicKey, 32))).toBe(true)
      expect(verifyKip5(`${message}?`, hexToBytes(signature, 64), hexToBytes(publicKey, 32))).toBe(false)
    }
  })

  it('uses BLAKE3 as the KCC-1 hash', () => {
    expect(bytesToHex(kccHash())).toBe('af1349b9f5f9a1a6a0404dea36dcc9499bcb25c9adc112b7cc9a93cae41f3262')
  })

  it('builds the same wallet message as Rust', () => {
    const record = (march as SignedRecord).record
    const message = attestationMessage(record, recordLeaf(record))
    expect(message.split('\n').slice(0, 5)).toEqual([
      'Veritas price attestation v1',
      'item: milk.whole.1l.dk',
      'amount_minor: 1195 DKK',
      'qty_milli: 1000 l',
      'observed_on: 2026-03-02 (live)',
    ])
  })
})

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
