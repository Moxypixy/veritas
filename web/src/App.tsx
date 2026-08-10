import { useState, type FormEvent } from 'react'

const REGIONS = [
  { id: 'euro-area', label: 'Euro area' },
  { id: 'united-states', label: 'United States' },
  { id: 'united-kingdom', label: 'United Kingdom' },
  { id: 'japan', label: 'Japan' },
] as const

export default function App() {
  const [consented, setConsented] = useState(false)
  const [employment, setEmployment] = useState<'employed' | 'unemployed'>('employed')
  const [depositConfirmed, setDepositConfirmed] = useState(false)
  const [privacyConfirmed, setPrivacyConfirmed] = useState(false)
  const [message, setMessage] = useState<string | null>(null)

  const durationLabel = employment === 'employed' ? 'How many months have you been employed?' : 'How many months have you been unemployed?'

  function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault()
    if (!depositConfirmed || !privacyConfirmed) {
      setMessage('Confirm the deposit and privacy statements before continuing.')
      return
    }
    setMessage('Your answers are ready for wallet approval. Wallet connection and transaction signing arrive in Task 10; no vote or deposit has been submitted.')
  }

  if (!consented) {
    return (
      <main className="page">
        <header className="hero">
          <p className="eyebrow">Cost-of-Living Pulse · Kaspa testnet</p>
          <h1>Before you vote</h1>
          <p>Read how your answer, commitment, and deposit are handled. You can continue only after acknowledging this notice.</p>
        </header>

        <section className="card" aria-labelledby="privacy-heading">
          <h2 id="privacy-heading">What is recorded</h2>
          <dl className="facts">
            <div>
              <dt>On Kaspa</dt>
              <dd>Your wallet identity as needed for the vote, UTC month, a cryptographic commitment to your answer, and the deposit lock. The commitment cannot be removed from Kaspa.</dd>
            </div>
            <div>
              <dt>On our servers</dt>
              <dd>Your full encrypted answer is used for anonymous aggregates. You can request its export or erasure later; erasure cannot remove the on-chain commitment.</dd>
            </div>
            <div>
              <dt>Deposit</dt>
              <dd>A fixed <strong>0.1 KAS</strong> testnet deposit is locked with a valid monthly vote and becomes claimable the following month when the rules are followed.</dd>
            </div>
          </dl>
          <aside className="notice">
            <h2>Important trust boundary</h2>
            <p>Option A relies on the indexer to gate one vote per wallet for the current UTC month and to aggregate answers honestly. On-chain records the deposit and commitment, not your plaintext answer.</p>
          </aside>
          <label className="check-row">
            <input type="checkbox" checked={consented} onChange={(event) => setConsented(event.target.checked)} />
            <span>I understand what is immutable on-chain, what can be erased off-chain, and how the 0.1 KAS deposit works.</span>
          </label>
        </section>
      </main>
    )
  }

  return (
    <main className="page">
      <header className="hero">
        <p className="eyebrow">Cost-of-Living Pulse · Kaspa testnet</p>
        <h1>Share this month’s experience</h1>
        <p>Your detailed answer is not published on-chain. Only its commitment and the deposit are intended for Kaspa.</p>
      </header>

      <form className="card form" onSubmit={handleSubmit}>
        <fieldset>
          <legend>About your household costs</legend>
          <label htmlFor="region">Region</label>
          <select id="region" name="region" defaultValue="" required>
            <option value="" disabled>Select a region</option>
            {REGIONS.map((region) => <option key={region.id} value={region.id}>{region.label}</option>)}
          </select>

          <label htmlFor="necessities">Necessities as a share of your wage</label>
          <input id="necessities" name="necessities" type="number" min="0" max="100" step="1" inputMode="numeric" required aria-describedby="necessities-help" />
          <p id="necessities-help" className="help">Enter a whole percentage from 0 to 100. Include housing, food, and petrol or fuel.</p>
        </fieldset>

        <fieldset>
          <legend>Employment</legend>
          <div className="radio-group">
            <label><input type="radio" name="employment" value="employed" checked={employment === 'employed'} onChange={() => setEmployment('employed')} /> Employed</label>
            <label><input type="radio" name="employment" value="unemployed" checked={employment === 'unemployed'} onChange={() => setEmployment('unemployed')} /> Unemployed</label>
          </div>
          <label htmlFor="duration">{durationLabel}</label>
          <input id="duration" name="duration" type="number" min="0" step="1" inputMode="numeric" required />
        </fieldset>

        <section className="deposit" aria-labelledby="deposit-heading">
          <h2 id="deposit-heading">Testnet deposit: 0.1 KAS</h2>
          <p>This amount is displayed before any future wallet approval. This Task does not connect a wallet, sign, or send a transaction.</p>
          <button type="button" className="secondary" onClick={() => setMessage('Wallet connection will be added in Task 10. No wallet is connected.')}>Connect wallet (coming soon)</button>
        </section>

        <fieldset>
          <legend>Confirm before review</legend>
          <label className="check-row">
            <input type="checkbox" checked={depositConfirmed} onChange={(event) => setDepositConfirmed(event.target.checked)} />
            <span>I understand that 0.1 KAS will be locked when I later approve a valid vote transaction.</span>
          </label>
          <label className="check-row">
            <input type="checkbox" checked={privacyConfirmed} onChange={(event) => setPrivacyConfirmed(event.target.checked)} />
            <span>I understand that the on-chain commitment cannot be erased, while the encrypted server-side answer may be erased.</span>
          </label>
        </fieldset>

        {message && <p className="status" role="status">{message}</p>}
        <button type="submit">Review vote</button>
      </form>
    </main>
  )
}
