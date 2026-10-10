import { useState, type FormEvent } from 'react'

import demoLog from '../../../examples/demo/anchor-log.json'
import demoRecord from '../../../examples/demo/milk-2026-03.signed.json'
import demoReceipt from '../../../examples/demo/receipts/milk-2026-03.receipt.json'
import demoAcc from '../../../examples/demo/live-acc.txt?raw'
import Navigation from '../Navigation'
import {
  BASE_UNIT,
  formatMilli,
  verify,
  type AnchorLog,
  type Receipt,
  type SignedRecord,
  type Verdict,
} from '../veritas/verify'

type Inputs = { record: string; receipt: string; log: string; acc: string }

const EMPTY: Inputs = { record: '', receipt: '', log: '', acc: '' }

const pretty = (value: unknown) => JSON.stringify(value, null, 2)

/** Formats minor units using the currency's own number of decimals (DKK 2, JPY 0). */
export function formatAmount(amountMinor: number, currency: string): string {
  try {
    const format = new Intl.NumberFormat('en', { style: 'currency', currency })
    const digits = format.resolvedOptions().maximumFractionDigits ?? 2
    return format.format(amountMinor / 10 ** digits)
  } catch {
    return `${amountMinor} ${currency} minor units`
  }
}

export function runVerification(inputs: Inputs): { verdict: Verdict; signed: SignedRecord | null } {
  let signed: SignedRecord
  let receipt: Receipt
  let log: AnchorLog
  try {
    signed = JSON.parse(inputs.record) as SignedRecord
    receipt = JSON.parse(inputs.receipt) as Receipt
    log = JSON.parse(inputs.log) as AnchorLog
  } catch {
    return { verdict: { ok: false, reason: 'One of the files is not valid JSON.' }, signed: null }
  }
  return { verdict: verify(signed, receipt, log, inputs.acc), signed }
}

export function VerdictView({ verdict, signed }: { verdict: Verdict; signed: SignedRecord | null }) {
  if (!verdict.ok) {
    return (
      <section className="verdict verdict-no" aria-labelledby="verdict-heading">
        <h2 id="verdict-heading">NO</h2>
        <p>{verdict.reason}</p>
      </section>
    )
  }
  const record = signed?.record
  return (
    <section className="verdict verdict-yes" aria-labelledby="verdict-heading">
      <h2 id="verdict-heading">YES</h2>
      {record && (
        <p>
          {record.item}: {formatAmount(record.amount_minor, record.currency)} for {record.qty_milli / 1000} {record.unit} on{' '}
          {record.observed_on}
          {record.place ? `, ${record.place}` : ''}.
        </p>
      )}
      <dl className="facts">
        {verdict.pricePerBaseMilli !== null && record && (
          <div>
            <dt>Unit price</dt>
            <dd>
              {formatMilli(verdict.pricePerBaseMilli)} {record.currency} minor units per {BASE_UNIT[record.unit]}
            </dd>
          </div>
        )}
        <div>
          <dt>What the date means</dt>
          <dd>{verdict.timeClaim}</dd>
        </div>
        <div>
          <dt>Anchor</dt>
          <dd>Batch {verdict.batchIndex} of the anchor log, which matches the live covenant root.</dd>
        </div>
        <div>
          <dt>Attester</dt>
          <dd className="mono">{verdict.attester}</dd>
        </div>
      </dl>
    </section>
  )
}

export default function VerifyPage() {
  const [inputs, setInputs] = useState<Inputs>(EMPTY)
  const [result, setResult] = useState<ReturnType<typeof runVerification> | null>(null)

  function update(field: keyof Inputs, value: string) {
    setInputs((current) => ({ ...current, [field]: value }))
    setResult(null)
  }

  async function loadFile(field: keyof Inputs, file: File | undefined) {
    if (file) update(field, await file.text())
  }

  function loadDemo() {
    setInputs({ record: pretty(demoRecord), receipt: pretty(demoReceipt), log: pretty(demoLog), acc: demoAcc.trim() })
    setResult(null)
  }

  function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault()
    setResult(runVerification(inputs))
  }

  const fields: { key: Exclude<keyof Inputs, 'acc'>; label: string; help: string }[] = [
    { key: 'record', label: 'Signed price record', help: 'The .signed.json file the attester keeps.' },
    { key: 'receipt', label: 'Receipt', help: 'The .receipt.json file issued when the record was batched.' },
    { key: 'log', label: 'Anchor log', help: 'The public list of batch roots. Anyone can mirror it; it must hash to the live root.' },
  ]

  return (
    <main className="page">
      <Navigation current="verify" />
      <header className="hero">
        <p className="eyebrow">Veritas · Kaspa testnet</p>
        <h1>Was this the price, then?</h1>
        <p>
          Check a price record against Kaspa. Everything runs in this browser: nothing is uploaded, and no Veritas server is involved.
        </p>
      </header>

      <form className="card form" onSubmit={handleSubmit}>
        <button type="button" className="secondary" onClick={loadDemo}>
          Load the milk demo
        </button>
        <p className="help">Try changing one digit of <code>amount_minor</code> after loading, then verify again.</p>

        {fields.map((field) => (
          <fieldset key={field.key}>
            <legend>{field.label}</legend>
            <label htmlFor={`${field.key}-file`}>Open a file</label>
            <input
              id={`${field.key}-file`}
              type="file"
              accept="application/json,.json"
              onChange={(event) => void loadFile(field.key, event.currentTarget.files?.[0])}
            />
            <label htmlFor={`${field.key}-text`}>Or paste it</label>
            <textarea
              id={`${field.key}-text`}
              aria-describedby={`${field.key}-help`}
              rows={6}
              spellCheck={false}
              value={inputs[field.key]}
              onChange={(event) => update(field.key, event.currentTarget.value)}
            />
            <p id={`${field.key}-help`} className="help">{field.help}</p>
          </fieldset>
        ))}

        <fieldset>
          <legend>Live anchor</legend>
          <label htmlFor="acc">Accumulator held by the anchor covenant UTXO</label>
          <input
            id="acc"
            type="text"
            inputMode="text"
            autoComplete="off"
            spellCheck={false}
            aria-describedby="acc-help"
            value={inputs.acc}
            onChange={(event) => update('acc', event.currentTarget.value)}
          />
          <p id="acc-help" className="help">
            64 hex characters, read from the anchor's current state on any Kaspa node. A later version will read it directly.
          </p>
        </fieldset>

        <button type="submit">Verify</button>

        <div role="status" aria-live="polite">
          {result && <VerdictView verdict={result.verdict} signed={result.signed} />}
        </div>
      </form>
    </main>
  )
}
