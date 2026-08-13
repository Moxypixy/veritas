import { useState, type FormEvent } from 'react'

import { commitmentHash } from './commitment'
import { walletAuthApi } from './wallet/api'
import {
  TESTNET_FAUCET_URL,
  connectAndAuthenticate,
  getKastleProvider,
  missingWalletMessage,
  walletErrorMessage,
  type WalletSession,
} from './wallet/kastle'
import { ChartPanel } from './pages/ChartPage'
import { REGIONS } from './regions'

export function VoteSection() {
  const [consented, setConsented] = useState(false)
  const [employment, setEmployment] = useState<'employed' | 'unemployed'>('employed')
  const [depositConfirmed, setDepositConfirmed] = useState(false)
  const [privacyConfirmed, setPrivacyConfirmed] = useState(false)
  const [message, setMessage] = useState<string | null>(null)
  const [walletSession, setWalletSession] = useState<WalletSession | null>(null)
  const [walletBusy, setWalletBusy] = useState(false)
  const [pendingCommitment, setPendingCommitment] = useState<string | null>(null)

  const durationLabel = employment === 'employed' ? 'How many months have you been employed?' : 'How many months have you been unemployed?'

  function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault()
    if (!consented || !depositConfirmed || !privacyConfirmed) {
      setMessage('Confirm the deposit and privacy statements before continuing.')
      return
    }
    if (!walletSession) {
      setMessage('Connect and authenticate a Kaspa Testnet-10 wallet before reviewing the transaction.')
      return
    }
    const form = new FormData(event.currentTarget)
    const salt = crypto.getRandomValues(new Uint8Array(16))
    const now = new Date()
    const monthKey = `${now.getUTCFullYear()}-${String(now.getUTCMonth() + 1).padStart(2, '0')}`
    try {
      const commitment = commitmentHash({
        regionId: String(form.get('region')),
        necessitiesPct: Number(form.get('necessities')),
        employed: form.get('employment') === 'employed',
        durationMonths: Number(form.get('duration')),
        monthKey,
        wallet: walletSession.wallet,
        salt,
      })
      setPendingCommitment(commitment)
      setMessage(`Commitment ${commitment.slice(0, 12)}… is ready. Transaction construction is not configured in this deployment, so no transaction can be signed or broadcast.`)
    } catch (error) {
      setMessage(error instanceof Error ? error.message : 'The vote commitment could not be built.')
    }
  }

  async function connectWallet() {
    const provider = getKastleProvider()
    if (!provider) {
      setMessage(missingWalletMessage())
      return
    }
    setWalletBusy(true)
    setMessage(null)
    try {
      const session = await connectAndAuthenticate(provider, walletAuthApi)
      setWalletSession(session)
      setMessage(
        `Wallet authenticated for ${session.wallet}. Fund Testnet-10 at ${TESTNET_FAUCET_URL} if the balance is empty.`,
      )
    } catch (error) {
      setMessage(walletErrorMessage(error))
    } finally {
      setWalletBusy(false)
    }
  }

  return (
    <section className="vote-section" aria-labelledby="vote-heading">
      <header className="section-heading">
        <h2 id="vote-heading">Share this month’s experience</h2>
        <p>Your detailed answer is not published on-chain. Only its commitment and the deposit are intended for Kaspa.</p>
      </header>

      <section className="card" aria-labelledby="privacy-heading">
        <h3 id="privacy-heading">Before you vote</h3>
        <p>Read how your answer, commitment, and deposit are handled. You can submit only after acknowledging this notice.</p>
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
          <h3>Important trust boundary</h3>
          <p>Option A relies on the indexer to gate one vote per wallet for the current UTC month and to aggregate answers honestly. On-chain records the deposit and commitment, not your plaintext answer.</p>
        </aside>
        <label className="check-row">
          <input type="checkbox" checked={consented} onChange={(event) => setConsented(event.target.checked)} />
          <span>I understand what is immutable on-chain, what can be erased off-chain, and how the 0.1 KAS deposit works.</span>
        </label>
      </section>

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
          <p>Before a transaction can be signed, you will see its destination, 0.1 KAS deposit, and commitment. Your wallet asks for approval; this app does not receive private keys or seed phrases.</p>
          <button type="button" className="secondary" onClick={connectWallet} disabled={walletBusy}>
            {walletBusy ? 'Connecting wallet…' : walletSession ? 'Wallet authenticated' : 'Connect Kastle (Testnet-10)'}
          </button>
          {walletSession && <p className="help">Connected: {walletSession.wallet}</p>}
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
        {pendingCommitment && (
          <section className="deposit" aria-labelledby="transaction-review-heading">
            <h2 id="transaction-review-heading">Transaction review</h2>
            <dl className="facts">
              <div><dt>Network</dt><dd>Kaspa Testnet-10 only</dd></div>
              <div><dt>Deposit</dt><dd>0.1 KAS (10,000,000 sompi)</dd></div>
              <div><dt>Commitment</dt><dd><code>{pendingCommitment}</code></dd></div>
              <div><dt>Destination</dt><dd>Awaiting an unsigned covenant transaction from the server.</dd></div>
            </dl>
            <p>The wallet will show its own signing approval. This application will only request signing after this review and will only broadcast after that approval.</p>
            <button type="button" disabled>Transaction template unavailable</button>
          </section>
        )}
        <button type="submit">Review vote</button>
      </form>
    </section>
  )
}

export default function App() {
  return (
    <main className="page">
      <header className="hero">
        <p className="eyebrow">Cost-of-Living Pulse · Kaspa testnet</p>
        <h1>Cost-of-Living Pulse</h1>
        <p>Compare price changes with anonymous monthly averages, then share your experience.</p>
      </header>
      <ChartPanel />
      <VoteSection />
    </main>
  )
}
