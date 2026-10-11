import { useState, type FormEvent } from 'react'

import Navigation from '../Navigation'
import {
  connectKaspire,
  missingWalletMessage,
  signKip5,
  waitForKaspire,
  walletErrorMessage,
  type KaspireAccount,
  type KaspireProvider,
} from '../wallet/kaspire'
import {
  UNIT_PICO,
  attestationMessage,
  bytesToHex,
  kccHash,
  defaultDisplayUnit,
  formatUnitPrice,
  pricePerUnitMilli,
  unitLabel,
  recordLeaf,
  validateRecord,
  type PriceRecord,
  type SignedRecord,
  type Source,
  type Unit,
} from '../veritas/verify'

const UNITS = Object.keys(UNIT_PICO) as Unit[]
const SOURCES: { value: Source; label: string }[] = [
  { value: 'live', label: 'Live: I saw this price today or recently' },
  { value: 'archive', label: 'Archive: from an independently archived web page' },
  { value: 'attested_late', label: 'Late: an old receipt I am recording now' },
]

export type Draft = {
  item: string
  amount: string
  currency: string
  quantity: string
  unit: Unit
  place: string
  observedOn: string
  source: Source
}

/** Parses "12.95" into 1295 using the currency's minor-unit digits. */
export function toMinorUnits(amount: string, currency: string): number {
  let digits = 2
  try {
    digits = new Intl.NumberFormat('en', { style: 'currency', currency }).resolvedOptions().maximumFractionDigits ?? 2
  } catch {
    // keep 2
  }
  const match = /^(\d+)(?:[.,](\d+))?$/.exec(amount.trim())
  if (!match || (match[2]?.length ?? 0) > digits) throw new Error(`Enter the price as a number with at most ${digits} decimals.`)
  const value = Number(match[1]) * 10 ** digits + Number((match[2] ?? '').padEnd(digits, '0') || 0)
  if (!Number.isSafeInteger(value)) throw new Error('That price is too large.')
  return value
}

/** Parses "0.5" into 500 thousandths. */
export function toQtyMilli(quantity: string): number {
  const match = /^(\d+)(?:[.,](\d{1,3}))?$/.exec(quantity.trim())
  if (!match) throw new Error('Enter the quantity as printed, with at most 3 decimals.')
  return Number(match[1]) * 1000 + Number((match[2] ?? '').padEnd(3, '0') || 0)
}

/** Builds the record exactly as Rust validates and hashes it. */
export function draftToRecord(draft: Draft, attester: string, salt: Uint8Array, evidence: Uint8Array | null): PriceRecord {
  const currency = draft.currency.trim().toUpperCase()
  const record: PriceRecord = {
    item: draft.item.trim(),
    amount_minor: toMinorUnits(draft.amount, currency),
    currency,
    qty_milli: toQtyMilli(draft.quantity),
    unit: draft.unit,
    place: draft.place.trim(),
    observed_on: draft.observedOn,
    source: draft.source,
    evidence: bytesToHex(evidence ?? new Uint8Array(32)),
    salt: bytesToHex(salt),
    attester,
  }
  const problem = validateRecord(record)
  if (problem) throw new Error(problem)
  return record
}

function downloadJson(name: string, value: unknown) {
  const url = URL.createObjectURL(new Blob([`${JSON.stringify(value, null, 2)}\n`], { type: 'application/json' }))
  const link = document.createElement('a')
  link.href = url
  link.download = name
  link.click()
  URL.revokeObjectURL(url)
}

const today = () => new Date().toISOString().slice(0, 10)

export default function AttestPage() {
  const [provider, setProvider] = useState<KaspireProvider | null>(null)
  const [account, setAccount] = useState<KaspireAccount | null>(null)
  const [draft, setDraft] = useState<Draft>({
    item: 'milk.whole.1l.dk',
    amount: '',
    currency: 'DKK',
    quantity: '1',
    unit: 'l',
    place: '',
    observedOn: today(),
    source: 'live',
  })
  const [evidence, setEvidence] = useState<Uint8Array | null>(null)
  const [signed, setSigned] = useState<SignedRecord | null>(null)
  const [message, setMessage] = useState<string | null>(null)
  const [busy, setBusy] = useState(false)

  function update<K extends keyof Draft>(key: K, value: Draft[K]) {
    setDraft((current) => ({ ...current, [key]: value }))
    setSigned(null)
  }

  async function connect() {
    setBusy(true)
    setMessage(null)
    try {
      const found = await waitForKaspire()
      if (!found) {
        setMessage(missingWalletMessage())
        return
      }
      setProvider(found)
      setAccount(await connectKaspire(found))
    } catch (error) {
      setMessage(walletErrorMessage(error))
    } finally {
      setBusy(false)
    }
  }

  async function hashEvidence(file: File | undefined) {
    setSigned(null)
    setEvidence(file ? kccHash(new Uint8Array(await file.arrayBuffer())) : null)
  }

  async function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault()
    if (!provider || !account) {
      setMessage('Connect Kaspire first; your attester key is your Kaspire public key.')
      return
    }
    setBusy(true)
    setMessage(null)
    try {
      const record = draftToRecord(draft, account.publicKey, crypto.getRandomValues(new Uint8Array(16)), evidence)
      const text = attestationMessage(record, recordLeaf(record))
      const signature = await signKip5(provider, account, text)
      setSigned({ record, signature: bytesToHex(signature) })
    } catch (error) {
      setMessage(error instanceof Error && !('code' in error) ? error.message : walletErrorMessage(error))
    } finally {
      setBusy(false)
    }
  }

  const displayUnit = signed ? defaultDisplayUnit(signed.record.unit, navigator.language) : null
  const unitPrice =
    signed && displayUnit ? pricePerUnitMilli(signed.record.amount_minor, signed.record.qty_milli, signed.record.unit, displayUnit) : null

  return (
    <main className="page">
      <Navigation current="attest" />
      <header className="hero">
        <p className="eyebrow">Veritas · Kaspire</p>
        <h1>Attest a price</h1>
        <p>
          Record what you paid. Kaspire signs it with your key (KIP-5); nothing is sent to a server. Keep the signed file:
          it is your evidence once it has been batched and anchored on Kaspa.
        </p>
      </header>

      <form className="card form" onSubmit={handleSubmit}>
        <section aria-labelledby="wallet-heading">
          <h2 id="wallet-heading">1. Your attester key</h2>
          <button type="button" className="secondary" onClick={connect} disabled={busy}>
            {account ? 'Kaspire connected' : busy ? 'Connecting…' : 'Connect Kaspire'}
          </button>
          {account && (
            <dl className="facts">
              <div>
                <dt>Account</dt>
                <dd className="mono">{account.address}</dd>
              </div>
              <div>
                <dt>Attester public key</dt>
                <dd className="mono">{account.publicKey}</dd>
              </div>
            </dl>
          )}
        </section>

        <fieldset>
          <legend>2. The price</legend>
          <label htmlFor="item">Item</label>
          <input id="item" type="text" required value={draft.item} aria-describedby="item-help" onChange={(e) => update('item', e.currentTarget.value)} />
          <p id="item-help" className="help">Catalogue id: lower-case letters, digits, dots or dashes, e.g. milk.whole.1l.dk.</p>

          <label htmlFor="amount">Price paid</label>
          <input id="amount" type="text" inputMode="decimal" required placeholder="12.95" value={draft.amount} onChange={(e) => update('amount', e.currentTarget.value)} />

          <label htmlFor="currency">Currency</label>
          <input id="currency" type="text" required maxLength={3} value={draft.currency} onChange={(e) => update('currency', e.currentTarget.value)} />

          <label htmlFor="quantity">Quantity as printed</label>
          <input id="quantity" type="text" inputMode="decimal" required value={draft.quantity} onChange={(e) => update('quantity', e.currentTarget.value)} />

          <label htmlFor="unit">Unit</label>
          <select id="unit" value={draft.unit} aria-describedby="unit-help" onChange={(e) => update('unit', e.currentTarget.value as Unit)}>
            {UNITS.map((unit) => (
              <option key={unit} value={unit}>{unit}</option>
            ))}
          </select>
          <p id="unit-help" className="help">us_pt and imp_pt differ by about 20%; pick the one on the label.</p>

          <label htmlFor="place">Place (optional)</label>
          <input id="place" type="text" placeholder="Shop and city, or a region" value={draft.place} onChange={(e) => update('place', e.currentTarget.value)} />

          <label htmlFor="observed">Date</label>
          <input id="observed" type="date" required value={draft.observedOn} onChange={(e) => update('observedOn', e.currentTarget.value)} />

          <label htmlFor="source">How you know</label>
          <select id="source" value={draft.source} onChange={(e) => update('source', e.currentTarget.value as Source)}>
            {SOURCES.map((source) => (
              <option key={source.value} value={source.value}>{source.label}</option>
            ))}
          </select>

          <label htmlFor="evidence">Receipt or shelf photo (optional)</label>
          <input id="evidence" type="file" accept="image/*,application/pdf" aria-describedby="evidence-help" onChange={(e) => void hashEvidence(e.currentTarget.files?.[0])} />
          <p id="evidence-help" className="help">Only its hash goes into the record. The photo stays on your device; keep it with the signed file.</p>
        </fieldset>

        <button type="submit" disabled={busy || !account}>Sign with Kaspire</button>

        <div role="status" aria-live="polite">
          {message && <p className="status">{message}</p>}
          {signed && (
            <section className="verdict verdict-yes" aria-labelledby="signed-heading">
              <h2 id="signed-heading">Signed</h2>
              <p>
                {signed.record.item}, {signed.record.observed_on}
                {unitPrice !== null && displayUnit && ` · ${formatUnitPrice(unitPrice, signed.record.currency)} per ${unitLabel(displayUnit)}`}
              </p>
              <button type="button" onClick={() => downloadJson(`${signed.record.item}-${signed.record.observed_on}.signed.json`, signed)}>
                Download signed record
              </button>
              <p className="help">Hand this file to a batcher (or run <code>veritas batch</code>) to anchor it and get your receipt.</p>
            </section>
          )}
        </div>
      </form>
    </main>
  )
}
