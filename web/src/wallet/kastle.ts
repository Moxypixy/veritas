export const TESTNET_10 = 'testnet-10'
export const TESTNET_FAUCET_URL = 'https://faucet-tn10.kaspanet.io/'
export const KASTLE_INSTALL_URL = 'https://docs.kastle.cc/'

export type KastleProvider = {
  connect(): Promise<boolean>
  getAccount(): Promise<{ address: string; publicKey: string }>
  getNetwork(): Promise<string>
  switchNetwork(networkId: string): Promise<string>
  signMessage(message: string): Promise<string>
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
  verifyChallenge(
    wallet: string,
    nonce: string,
    signature: string,
    publicKey: string,
  ): Promise<{ session_token: string }>
}

export function getKastleProvider(): KastleProvider | null {
  return (window as Window & { kastle?: KastleProvider }).kastle ?? null
}

export function missingWalletMessage(): string {
  return `No Kastle wallet was found. Install Kastle (${KASTLE_INSTALL_URL}) and reopen this page in a browser or wallet webview that provides window.kastle.`
}

export async function connectAndAuthenticate(
  provider: KastleProvider,
  api: AuthApi,
): Promise<WalletSession> {
  const connected = await provider.connect()
  if (!connected) {
    throw new Error('The wallet did not grant a connection.')
  }

  const account = await provider.getAccount()
  const wallet = account.address?.trim()
  if (!wallet) {
    throw new Error('The wallet did not provide an account.')
  }

  if ((await provider.getNetwork()) !== TESTNET_10) {
    await provider.switchNetwork(TESTNET_10)
    if ((await provider.getNetwork()) !== TESTNET_10) {
      throw new Error('Switch the wallet to Kaspa Testnet-10 before continuing.')
    }
  }

  const challenge = await api.createChallenge(wallet)
  const signature = await provider.signMessage(challenge.message)
  const verified = await api.verifyChallenge(
    wallet,
    challenge.nonce,
    signature,
    account.publicKey,
  )
  return { wallet, sessionToken: verified.session_token }
}

export function walletErrorMessage(error: unknown): string {
  const detail = error instanceof Error ? error.message : ''
  if (/reject|deny|cancel/i.test(detail)) {
    return 'Wallet approval was rejected. No signature or transaction was submitted.'
  }
  if (/testnet/i.test(detail)) return detail
  if (/account|connection/i.test(detail)) return detail
  return 'The wallet request failed. Check the wallet connection and try again.'
}
