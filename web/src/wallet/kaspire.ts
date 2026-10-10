// Kaspire wallet integration (KaspaHUB21, https://kaspire.kaslab.space/).
//
// The Kaspire browser extension injects an origin-bound `window.kaspire`
// provider with `request({ method, params })`. Veritas uses only:
//   requestAccounts, getPublicKey, getNetwork  -> who is attesting / logging in
//   signMessage { address, message }           -> KIP-5 personal signature
// Every signature needs fresh approval in Kaspire; connecting grants nothing.

import { bytesToHex, hexToBytes, verifyKip5 } from '../veritas/verify'

export const TESTNET_10 = 'testnet-10'
export const TESTNET_FAUCET_URL = 'https://faucet-tn10.kaspanet.io/'
export const KASPIRE_URL = 'https://kaspire.kaslab.space/'

export type KaspireRequest = { method: string; params?: unknown }

export type KaspireProvider = {
  isKaspire: boolean
  version?: string
  request(args: KaspireRequest): Promise<unknown>
  on?(event: string, listener: (...args: unknown[]) => void): void
  removeListener?(event: string, listener: (...args: unknown[]) => void): void
}

export type KaspireAccount = {
  address: string
  /** 32-byte x-only Schnorr public key, lower-case hex. */
  publicKey: string
  network: string
}

export type Challenge = {
  nonce: string
  expires_at: string
  message: string
}

export type WalletSession = {
  wallet: string
  sessionToken: string
}

type AuthApi = {
  createChallenge(wallet: string): Promise<Challenge>
  verifyChallenge(wallet: string, nonce: string, signature: string, publicKey: string): Promise<{ session_token: string }>
}

export function getKaspireProvider(): KaspireProvider | null {
  const provider = (window as Window & { kaspire?: KaspireProvider }).kaspire
  return provider?.isKaspire ? provider : null
}

/** Waits briefly for the extension's `kaspire#initialized` event. */
export function waitForKaspire(timeoutMs = 1500): Promise<KaspireProvider | null> {
  const existing = getKaspireProvider()
  if (existing) return Promise.resolve(existing)
  return new Promise((resolve) => {
    const done = () => {
      window.removeEventListener('kaspire#initialized', done)
      clearTimeout(timer)
      resolve(getKaspireProvider())
    }
    const timer = setTimeout(done, timeoutMs)
    window.addEventListener('kaspire#initialized', done)
  })
}

export function missingWalletMessage(): string {
  return `Kaspire was not found. Install the Kaspire browser extension (${KASPIRE_URL}) and reload this page.`
}

function normalizePublicKey(raw: unknown): string {
  const text = String(raw ?? '').trim().replace(/^0x/i, '').toLowerCase()
  // Accept a 33-byte compressed key by dropping its parity byte.
  const xOnly = text.length === 66 && /^0[23]/.test(text) ? text.slice(2) : text
  if (!/^[0-9a-f]{64}$/.test(xOnly)) throw new Error('Kaspire did not return a 32-byte public key.')
  return xOnly
}

/** Kaspire returns the KIP-5 signature as text; accept hex or base64 of 64 bytes. */
export function normalizeSignature(raw: unknown): Uint8Array {
  const text = String(raw ?? '').trim()
  const hex = text.replace(/^0x/i, '')
  if (/^[0-9a-fA-F]{128}$/.test(hex)) return hexToBytes(hex, 64)
  try {
    const bytes = Uint8Array.from(atob(text), (c) => c.charCodeAt(0))
    if (bytes.length === 64) return bytes
  } catch {
    // fall through
  }
  throw new Error('Kaspire returned a signature in an unexpected format.')
}

/** Asks the user to connect this site, then reads address, key and network. */
export async function connectKaspire(provider: KaspireProvider): Promise<KaspireAccount> {
  const accounts = (await provider.request({ method: 'requestAccounts' })) as string[] | undefined
  const address = accounts?.[0]?.trim()
  if (!address) throw new Error('Kaspire did not provide an account.')
  const publicKey = normalizePublicKey(await provider.request({ method: 'getPublicKey' }))
  const network = String(await provider.request({ method: 'getNetwork' }))
  return { address, publicKey, network }
}

/** KIP-5 personal signature, checked locally before it is used anywhere. */
export async function signKip5(provider: KaspireProvider, account: KaspireAccount, message: string): Promise<Uint8Array> {
  const result = (await provider.request({ method: 'signMessage', params: { address: account.address, message } })) as
    | { signature?: unknown }
    | string
  const signature = normalizeSignature(typeof result === 'string' ? result : result?.signature)
  if (!verifyKip5(message, signature, hexToBytes(account.publicKey, 32))) {
    throw new Error('The signature from Kaspire does not verify for this account.')
  }
  return signature
}

/** Pulse login: Testnet-10 account + KIP-5 signature over a server challenge. */
export async function connectAndAuthenticate(provider: KaspireProvider, api: AuthApi): Promise<WalletSession> {
  const account = await connectKaspire(provider)
  if (account.network !== TESTNET_10) {
    throw new Error('Switch Kaspire to Kaspa Testnet-10 before continuing.')
  }
  const challenge = await api.createChallenge(account.address)
  const signature = await signKip5(provider, account, challenge.message)
  const verified = await api.verifyChallenge(account.address, challenge.nonce, bytesToHex(signature), account.publicKey)
  return { wallet: account.address, sessionToken: verified.session_token }
}

/** Maps Kaspire's provider error codes to plain language. */
export function walletErrorMessage(error: unknown): string {
  const code = (error as { code?: unknown } | null)?.code
  const detail = error instanceof Error ? error.message : String((error as { message?: unknown } | null)?.message ?? '')
  if (code === 4001 || /reject|deny|cancel/i.test(detail)) {
    return 'The request was rejected in Kaspire. Nothing was signed or submitted.'
  }
  if (code === 4100) return 'Kaspire is locked or this site is not connected. Unlock Kaspire and connect again.'
  if (code === 4200) return 'This Kaspire version does not support that request or network.'
  if (/testnet|public key|account|signature|kaspire/i.test(detail)) return detail
  return 'The Kaspire request failed. Check the wallet and try again.'
}
