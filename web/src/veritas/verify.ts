// Browser verifier for Veritas price records. Mirrors the Rust crate
// (src/record.rs, src/kip5.rs, src/merkle.rs, src/accumulator.rs,
// src/verify.rs) byte for byte; tests run both against examples/demo so they
// cannot drift apart.
//
// Commitments use the KCC-1 Hash Function (BLAKE3-256). Attester signatures
// are KIP-5 personal-message signatures, which Kaspire's signMessage produces.

import { blake2b } from '@noble/hashes/blake2.js'
import { blake3 } from '@noble/hashes/blake3.js'
import { schnorr } from '@noble/curves/secp256k1.js'

export type Unit = 'l' | 'ml' | 'us_gal' | 'us_pt' | 'imp_gal' | 'imp_pt' | 'kg' | 'g' | 'lb' | 'oz' | 'each'
export type Source = 'live' | 'archive' | 'attested_late'

export type PriceRecord = {
  item: string
  amount_minor: number
  currency: string
  qty_milli: number
  unit: Unit
  place: string
  observed_on: string
  source: Source
  evidence: string
  salt: string
  attester: string
}

export type SignedRecord = { record: PriceRecord; signature: string }
export type ProofStep = { side: 'left' | 'right'; hash: string }
export type Receipt = {
  version: number
  batch_index: number
  batch_root: string
  leaf_index: number
  proof: ProofStep[]
  anchor_txid?: string
}
export type AnchorLog = { batch_roots: string[] }

export type Verdict =
  | {
      ok: true
      batchIndex: number
      attester: string
      observedOn: string
      source: Source
      pricePerBaseMilli: bigint | null
      timeClaim: string
    }
  | { ok: false; reason: string }

const RECORD_DOMAIN = 'veritas.price.v1'
const RECEIPT_VERSION = 1
const encoder = new TextEncoder()

// Size of one unit in pico-base units (10^-12 litre, kilogram or item). Exact.
export const UNIT_PICO: Record<Unit, bigint> = {
  l: 1_000_000_000_000n,
  ml: 1_000_000_000n,
  us_gal: 3_785_411_784_000n,
  us_pt: 473_176_473_000n,
  imp_gal: 4_546_090_000_000n,
  imp_pt: 568_261_250_000n,
  kg: 1_000_000_000_000n,
  g: 1_000_000_000n,
  lb: 453_592_370_000n,
  oz: 28_349_523_125n,
  each: 1_000_000_000_000n,
}

export const BASE_UNIT: Record<Unit, 'litre' | 'kg' | 'item'> = {
  l: 'litre', ml: 'litre', us_gal: 'litre', us_pt: 'litre', imp_gal: 'litre', imp_pt: 'litre',
  kg: 'kg', g: 'kg', lb: 'kg', oz: 'kg',
  each: 'item',
}

const SOURCE_CODE: Record<Source, number> = { live: 0, archive: 1, attested_late: 2 }

const TIME_CLAIM: Record<Source, string> = {
  live: 'Price attested at or near the observation date.',
  archive: "Price shown on an independently archived page at the archive's timestamp.",
  attested_late: 'Record existed by its anchoring time; the printed date is not vouched for.',
}

export function hexToBytes(hex: string, length: number): Uint8Array {
  const clean = hex.trim()
  if (clean.length !== length * 2 || !/^[0-9a-fA-F]*$/.test(clean)) {
    throw new Error(`expected ${length * 2} hex characters`)
  }
  const out = new Uint8Array(length)
  for (let i = 0; i < length; i++) out[i] = Number.parseInt(clean.slice(i * 2, i * 2 + 2), 16)
  return out
}

export function bytesToHex(bytes: Uint8Array): string {
  return Array.from(bytes, (b) => b.toString(16).padStart(2, '0')).join('')
}

function concat(...parts: Uint8Array[]): Uint8Array {
  const out = new Uint8Array(parts.reduce((n, p) => n + p.length, 0))
  let offset = 0
  for (const part of parts) {
    out.set(part, offset)
    offset += part.length
  }
  return out
}

/** KCC-1 Hash: unkeyed BLAKE3-256 of the concatenated parts. */
export function kccHash(...parts: Uint8Array[]): Uint8Array {
  return blake3(concat(...parts))
}

const hash = kccHash

const KIP5_KEY = encoder.encode('PersonalMessageSigningHash')

/** KIP-5: BLAKE2b-256 keyed with "PersonalMessageSigningHash". */
export function kip5Digest(message: string): Uint8Array {
  return blake2b(encoder.encode(message), { dkLen: 32, key: KIP5_KEY })
}

/** Verifies a KIP-5 personal-message signature against an x-only public key. */
export function verifyKip5(message: string, signature: Uint8Array, publicKey: Uint8Array): boolean {
  try {
    return schnorr.verify(signature, kip5Digest(message), publicKey)
  } catch {
    return false
  }
}

/** The text an attester signs; must match `attestation_message` in src/record.rs. */
export function attestationMessage(record: PriceRecord, leaf: Uint8Array): string {
  return [
    'Veritas price attestation v1',
    `item: ${record.item}`,
    `amount_minor: ${record.amount_minor} ${record.currency}`,
    `qty_milli: ${record.qty_milli} ${record.unit}`,
    `observed_on: ${record.observed_on} (${record.source})`,
    `leaf: ${bytesToHex(leaf)}`,
  ].join('\n')
}

function u32le(value: number): Uint8Array {
  const out = new Uint8Array(4)
  new DataView(out.buffer).setUint32(0, value, true)
  return out
}

function u64le(value: number): Uint8Array {
  const out = new Uint8Array(8)
  new DataView(out.buffer).setBigUint64(0, BigInt(value), true)
  return out
}

function lp(bytes: Uint8Array): Uint8Array {
  return concat(u32le(bytes.length), bytes)
}

function isValidDate(text: string): boolean {
  const match = /^(\d{4})-(\d{2})-(\d{2})$/.exec(text)
  if (!match) return false
  const [year, month, day] = match.slice(1).map(Number)
  const leap = (year % 4 === 0 && year % 100 !== 0) || year % 400 === 0
  const days = [31, leap ? 29 : 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31][month - 1]
  return year >= 1900 && days !== undefined && day >= 1 && day <= days
}

/** Returns a problem description, or null when the record is well formed. */
export function validateRecord(record: PriceRecord): string | null {
  if (!/^[a-z0-9._-]{1,64}$/.test(record.item)) return "item must be 1-64 chars of a-z, 0-9, '.', '_' or '-'"
  if (!Number.isSafeInteger(record.amount_minor) || record.amount_minor < 0) return 'amount must be a whole number of minor units'
  if (!/^[A-Z]{3}$/.test(record.currency)) return 'currency must be a 3-letter upper-case ISO 4217 code'
  if (!Number.isSafeInteger(record.qty_milli) || record.qty_milli <= 0) return 'quantity must be greater than zero'
  if (!(record.unit in UNIT_PICO)) return `unknown unit "${record.unit}"`
  if (encoder.encode(record.place ?? '').length > 128) return 'place must be at most 128 bytes'
  if (!isValidDate(record.observed_on)) return 'observed_on must be a real date in YYYY-MM-DD form'
  if (!(record.source in SOURCE_CODE)) return `unknown source "${record.source}"`
  return null
}

export function canonicalBytes(record: PriceRecord): Uint8Array {
  const problem = validateRecord(record)
  if (problem) throw new Error(problem)
  return concat(
    lp(encoder.encode(RECORD_DOMAIN)),
    lp(encoder.encode(record.item)),
    u64le(record.amount_minor),
    encoder.encode(record.currency),
    u64le(record.qty_milli),
    lp(encoder.encode(record.unit)),
    lp(encoder.encode(record.place ?? '')),
    encoder.encode(record.observed_on),
    Uint8Array.of(SOURCE_CODE[record.source]),
    hexToBytes(record.evidence, 32),
    hexToBytes(record.salt, 16),
    hexToBytes(record.attester, 32),
  )
}

export function recordLeaf(record: PriceRecord): Uint8Array {
  return hash(canonicalBytes(record))
}

export function leafNode(leaf: Uint8Array): Uint8Array {
  return hash(Uint8Array.of(0x00), leaf)
}

export function parentNode(left: Uint8Array, right: Uint8Array): Uint8Array {
  return hash(Uint8Array.of(0x01), left, right)
}

export function rootFromProof(leaf: Uint8Array, proof: ProofStep[]): Uint8Array {
  return proof.reduce((node, step) => {
    const sibling = hexToBytes(step.hash, 32)
    return step.side === 'left' ? parentNode(sibling, node) : parentNode(node, sibling)
  }, leafNode(leaf))
}

/** acc_0 = 0^32, acc_{n+1} = Hash(acc_n || root_n); same as anchor.sil. */
export function foldBatchRoots(roots: Uint8Array[]): Uint8Array {
  return roots.reduce((acc, root) => hash(acc, root), new Uint8Array(32))
}

/** Price per litre, kilogram or item in thousandths of the minor unit, rounded half up. */
export function pricePerBaseMilli(amountMinor: number, qtyMilli: number, unit: Unit): bigint | null {
  const numerator = BigInt(amountMinor) * 1_000_000_000_000_000_000n
  const denominator = BigInt(qtyMilli) * UNIT_PICO[unit]
  if (denominator === 0n) return null
  return (numerator + denominator / 2n) / denominator
}

export type Dimension = 'volume' | 'mass' | 'count'

export const UNIT_DIMENSION: Record<Unit, Dimension> = {
  l: 'volume', ml: 'volume', us_gal: 'volume', us_pt: 'volume', imp_gal: 'volume', imp_pt: 'volume',
  kg: 'mass', g: 'mass', lb: 'mass', oz: 'mass',
  each: 'count',
}

/** Units a reader can choose to see prices in, per dimension. */
export const DISPLAY_UNITS: Record<Dimension, { unit: Unit; label: string }[]> = {
  volume: [
    { unit: 'l', label: 'litre' },
    { unit: 'us_gal', label: 'US gallon' },
    { unit: 'us_pt', label: 'US pint' },
    { unit: 'imp_gal', label: 'imperial gallon' },
    { unit: 'imp_pt', label: 'imperial pint' },
  ],
  mass: [
    { unit: 'kg', label: 'kilogram' },
    { unit: 'lb', label: 'pound' },
    { unit: 'oz', label: 'ounce' },
  ],
  count: [{ unit: 'each', label: 'item' }],
}

export function unitLabel(unit: Unit): string {
  return DISPLAY_UNITS[UNIT_DIMENSION[unit]].find((option) => option.unit === unit)?.label ?? unit
}

/** Default display unit for a reader. US readers get US gallons and pounds,
 *  as US official statistics report milk per gallon. Everyone else gets litres
 *  and kilograms; UK rules require unit prices per litre or kilogram, with
 *  pints only as a supplementary figure. Readers can always switch. */
export function defaultDisplayUnit(unit: Unit, locale: string | undefined): Unit {
  const dimension = UNIT_DIMENSION[unit]
  let region: string | undefined
  try {
    // Only an explicit region counts; a bare language such as "en" stays metric.
    region = locale ? new Intl.Locale(locale).region : undefined
  } catch {
    region = undefined
  }
  if (dimension === 'volume') return region === 'US' ? 'us_gal' : 'l'
  if (dimension === 'mass') return region === 'US' ? 'lb' : 'kg'
  return 'each'
}

/** Price per one `display` unit in thousandths of the minor unit, rounded half
 *  up; must match `price_per_unit_milli` in src/units.rs. Null when the
 *  quantity is zero or the units measure different things. */
export function pricePerUnitMilli(amountMinor: number, qtyMilli: number, unit: Unit, display: Unit): bigint | null {
  if (UNIT_DIMENSION[unit] !== UNIT_DIMENSION[display]) return null
  const numerator = BigInt(amountMinor) * 1_000_000n * UNIT_PICO[display]
  const denominator = BigInt(qtyMilli) * UNIT_PICO[unit]
  if (denominator === 0n) return null
  return (numerator + denominator / 2n) / denominator
}

/** Formats a milli-minor-unit price in the currency's own notation. */
export function formatUnitPrice(milli: bigint, currency: string): string {
  try {
    const format = new Intl.NumberFormat('en', { style: 'currency', currency })
    const digits = format.resolvedOptions().maximumFractionDigits ?? 2
    // Round to whole minor units in bigint, then format an exact decimal
    // string, so large prices never pass through a lossy Number.
    const minor = (milli + 500n) / 1000n
    const scale = 10n ** BigInt(digits)
    const decimal = digits === 0 ? `${minor}` : `${minor / scale}.${(minor % scale).toString().padStart(digits, '0')}`
    return format.format(decimal as Intl.StringNumericLiteral)
  } catch {
    return `${formatMilli(milli)} ${currency} minor units`
  }
}

export function formatMilli(milli: bigint): string {
  return `${milli / 1000n}.${(milli % 1000n).toString().padStart(3, '0')}`
}

function sameBytes(a: Uint8Array, b: Uint8Array): boolean {
  return a.length === b.length && a.every((byte, i) => byte === b[i])
}

/** Was this the price, then? */
export function verify(signed: SignedRecord, receipt: Receipt, log: AnchorLog, liveAccHex: string): Verdict {
  let leaf: Uint8Array
  try {
    leaf = recordLeaf(signed.record)
  } catch (error) {
    return { ok: false, reason: `Record is malformed: ${error instanceof Error ? error.message : String(error)}` }
  }

  let signatureOk = false
  try {
    signatureOk = verifyKip5(
      attestationMessage(signed.record, leaf),
      hexToBytes(signed.signature, 64),
      hexToBytes(signed.record.attester, 32),
    )
  } catch {
    signatureOk = false
  }
  if (!signatureOk) return { ok: false, reason: 'Signature does not match the record and attester.' }

  if (receipt.version !== RECEIPT_VERSION) return { ok: false, reason: `Receipt version ${receipt.version} is not supported.` }

  let batchRoot: Uint8Array
  let roots: Uint8Array[]
  let liveAcc: Uint8Array
  try {
    batchRoot = hexToBytes(receipt.batch_root, 32)
    roots = log.batch_roots.map((root) => hexToBytes(root, 32))
    liveAcc = hexToBytes(liveAccHex, 32)
    if (!sameBytes(rootFromProof(leaf, receipt.proof), batchRoot)) {
      return { ok: false, reason: 'Record is not in the batch its receipt names.' }
    }
  } catch (error) {
    return { ok: false, reason: `Receipt or anchor data is malformed: ${error instanceof Error ? error.message : String(error)}` }
  }

  const listed = roots[receipt.batch_index]
  if (!listed || !sameBytes(listed, batchRoot)) return { ok: false, reason: 'Batch root is not in the anchor log at that position.' }
  if (!sameBytes(foldBatchRoots(roots), liveAcc)) return { ok: false, reason: 'Anchor log does not hash to the live covenant root.' }

  const record = signed.record
  return {
    ok: true,
    batchIndex: receipt.batch_index,
    attester: record.attester,
    observedOn: record.observed_on,
    source: record.source,
    pricePerBaseMilli: pricePerBaseMilli(record.amount_minor, record.qty_milli, record.unit),
    timeClaim: TIME_CLAIM[record.source],
  }
}
