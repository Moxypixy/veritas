import { blake2b } from '@noble/hashes/blake2.js'

const COMMITMENT_DOMAIN = 'veritas.colp.v1'
const encoder = new TextEncoder()

export type VoteAnswer = {
  regionId: string
  necessitiesPct: number
  employed: boolean
  durationMonths: number
  monthKey: string
  wallet: string
  salt: Uint8Array
}

export function canonicalBytes(answer: VoteAnswer): Uint8Array {
  validateAnswer(answer)

  return concatBytes(
    lengthPrefixed(COMMITMENT_DOMAIN),
    lengthPrefixed(answer.regionId),
    Uint8Array.of(answer.necessitiesPct, Number(answer.employed)),
    uint32Le(answer.durationMonths),
    lengthPrefixed(answer.monthKey),
    lengthPrefixed(answer.wallet),
    answer.salt,
  )
}

export function commitmentHash(answer: VoteAnswer): string {
  return bytesToHex(blake2b(canonicalBytes(answer), { dkLen: 32 }))
}

function validateAnswer(answer: VoteAnswer): void {
  if (!Number.isInteger(answer.necessitiesPct) || answer.necessitiesPct < 0 || answer.necessitiesPct > 100) {
    throw new Error('necessities percentage must be between 0 and 100')
  }
  if (answer.regionId.length === 0) {
    throw new Error('region ID must not be empty')
  }
  if (!/^[\x00-\x7F]+$/.test(answer.regionId)) {
    throw new Error('region ID must contain only ASCII')
  }
  if (!/^\d{4}-\d{2}$/.test(answer.monthKey)) {
    throw new Error('month key must match YYYY-MM')
  }
  if (answer.wallet.length === 0) {
    throw new Error('wallet must not be empty')
  }
  if (!Number.isInteger(answer.durationMonths) || answer.durationMonths < 0 || answer.durationMonths > 0xffff_ffff) {
    throw new Error('duration months must fit an unsigned 32-bit integer')
  }
  if (answer.salt.length !== 16) {
    throw new Error('salt must be 16 bytes')
  }
}

function lengthPrefixed(value: string): Uint8Array {
  const bytes = encoder.encode(value)
  return concatBytes(uint32Le(bytes.length), bytes)
}

function uint32Le(value: number): Uint8Array {
  const output = new Uint8Array(4)
  new DataView(output.buffer).setUint32(0, value, true)
  return output
}

function concatBytes(...parts: Uint8Array[]): Uint8Array {
  const output = new Uint8Array(parts.reduce((total, part) => total + part.length, 0))
  let offset = 0
  for (const part of parts) {
    output.set(part, offset)
    offset += part.length
  }
  return output
}

function bytesToHex(bytes: Uint8Array): string {
  return Array.from(bytes, (byte) => byte.toString(16).padStart(2, '0')).join('')
}
