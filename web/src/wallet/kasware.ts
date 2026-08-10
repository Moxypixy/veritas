export const TESTNET_10 = 'kaspa_testnet_10'

export type KaswareProvider = {
  requestAccounts(): Promise<string[]>
  getNetwork(): Promise<string>
  switchNetwork(network: string): Promise<void>
  getPublicKey(): Promise<string>
  signMessage(message: string, params?: { type: 'schnorr' }): Promise<string>
  signPskt(request: { txJsonString: string; options?: { signInputs?: Array<{ index: number; sighashType: number }> } }): Promise<string>
  pushTx(txJsonString: string): Promise<string>
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

export function getKaswareProvider(): KaswareProvider | null {
  return (window as Window & { kasware?: KaswareProvider }).kasware ?? null
}

export async function connectAndAuthenticate(provider: KaswareProvider, api: AuthApi): Promise<WalletSession> {
  const accounts = await provider.requestAccounts()
  const wallet = accounts[0]
  if (!wallet) throw new Error('The wallet did not provide an account.')

  if (await provider.getNetwork() !== TESTNET_10) {
    await provider.switchNetwork(TESTNET_10)
    if (await provider.getNetwork() !== TESTNET_10) {
      throw new Error('Switch the wallet to Kaspa Testnet-10 before continuing.')
    }
  }

  const challenge = await api.createChallenge(wallet)
  const signature = await provider.signMessage(challenge.message, { type: 'schnorr' })
  const publicKey = await provider.getPublicKey()
  const verified = await api.verifyChallenge(wallet, challenge.nonce, signature, publicKey)
  return { wallet, sessionToken: verified.session_token }
}

export function walletErrorMessage(error: unknown): string {
  const detail = error instanceof Error ? error.message : ''
  if (/reject|deny|cancel/i.test(detail)) {
    return 'Wallet approval was rejected. No signature or transaction was submitted.'
  }
  if (/testnet/i.test(detail)) return detail
  if (/account/i.test(detail)) return detail
  return 'The wallet request failed. Check the wallet connection and try again.'
}
