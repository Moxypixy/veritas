import { renderToStaticMarkup } from 'react-dom/server'
import { describe, expect, it } from 'vitest'

import log from '../../../examples/demo/anchor-log.json'
import record from '../../../examples/demo/milk-us-gallon.signed.json'
import receipt from '../../../examples/demo/receipts/milk-us-gallon.receipt.json'
import acc from '../../../examples/demo/live-acc.txt?raw'

import { VerdictView, formatAmount, runVerification } from './VerifyPage'

const demo = { record: JSON.stringify(record), receipt: JSON.stringify(receipt), log: JSON.stringify(log), acc: acc.trim() }

describe('VerifyPage', () => {
  it('shows YES with the per-litre price for the US gallon demo', () => {
    const result = runVerification(demo)
    const html = renderToStaticMarkup(<VerdictView verdict={result.verdict} signed={result.signed} />)
    expect(html).toContain('YES')
    expect(html).toContain('113.858 USD minor units per litre')
    expect(html).toContain('independently archived page')
  })

  it('shows NO with a reason when the record is edited', () => {
    const result = runVerification({ ...demo, record: demo.record.replace('"amount_minor":431', '"amount_minor":331') })
    const html = renderToStaticMarkup(<VerdictView verdict={result.verdict} signed={result.signed} />)
    expect(html).toContain('NO')
    expect(html).toContain('Signature does not match')
  })

  it('handles invalid JSON', () => {
    expect(runVerification({ ...demo, receipt: '{' }).verdict).toEqual({ ok: false, reason: 'One of the files is not valid JSON.' })
  })
})

describe('formatAmount', () => {
  it('uses each currency’s minor-unit digits', () => {
    expect(formatAmount(1295, 'DKK')).toMatch(/^DKK\s12\.95$/u)
    expect(formatAmount(431, 'USD')).toBe('$4.31')
    expect(formatAmount(250, 'JPY')).toBe('¥250')
  })
})
