import { schnorr } from '@noble/curves/secp256k1.js'
import { describe, expect, it, vi } from 'vitest'

import { bytesToHex, kip5Digest } from '../veritas/verify'
import {
  TESTNET_10,
  connectAndAuthenticate,
  connectKaspire,
  missingWalletMessage,
  normalizeSignature,
  signKip5,
  walletErrorMessage,
  type KaspireProvider,
} from './kaspire'

const secret = new Uint8Array(32).fill(3)
const publicKey = bytesToHex(schnorr.getPublicKey(secret))
const address = 'kaspatest:qpveritasdemoaddress'

/** A stand-in for window.kaspire that signs KIP-5 like the real wallet. */
function mockKaspire(overrides: Record<string, (params?: unknown) => unknown> = {}): KaspireProvider {
  const handlers: Record<string, (params?: unknown) => unknown> = {
    requestAccounts: () => [address],
    getPublicKey: () => publicKey,
    getNetwork: () => TESTNET_10,
    signMessage: (params) => {
      const { message } = params as { address: string; message: string }
      return { address, signature: bytesToHex(schnorr.sign(kip5Digest(message), secret)) }
    },
    ...overrides,
  }
  return {
    isKaspire: true,
    request: vi.fn(async ({ method, params }) => {
      const handler = handlers[method]
      if (!handler) throw Object.assign(new Error('unsupported'), { code: 4200 })
      return handler(params)
    }),
  }
}

function mockApi() {
  return {
    createChallenge: vi.fn(async () => ({ nonce: 'n1', expires_at: '2026-10-10T16:00:00Z', message: 'Veritas login\nnonce=n1' })),
    verifyChallenge: vi.fn(async () => ({ session_token: 'session-1' })),
  }
}

describe('Kaspire wallet', () => {
  it('logs in with a locally verified KIP-5 signature', async () => {
    const api = mockApi()
    const session = await connectAndAuthenticate(mockKaspire(), api)
    expect(session).toEqual({ wallet: address, sessionToken: 'session-1' })
    const [wallet, nonce, signature, key] = api.verifyChallenge.mock.calls[0] as unknown as string[]
    expect([wallet, nonce, key]).toEqual([address, 'n1', publicKey])
    expect(signature).toMatch(/^[0-9a-f]{128}$/)
  })

  it('stops before asking the server when Kaspire is not on Testnet-10', async () => {
    const api = mockApi()
    await expect(connectAndAuthenticate(mockKaspire({ getNetwork: () => 'mainnet' }), api)).rejects.toThrow(/Testnet-10/)
    expect(api.createChallenge).not.toHaveBeenCalled()
  })

  it('refuses a signature that does not verify for the account', async () => {
    const provider = mockKaspire({ signMessage: () => ({ signature: '11'.repeat(64) }) })
    const account = await connectKaspire(provider)
    await expect(signKip5(provider, account, 'hello')).rejects.toThrow(/does not verify/)
  })

  it('accepts hex or base64 signatures and compressed public keys', async () => {
    const bytes = new Uint8Array(64).fill(7)
    expect(normalizeSignature(`0x${bytesToHex(bytes)}`)).toEqual(bytes)
    expect(normalizeSignature(btoa(String.fromCharCode(...bytes)))).toEqual(bytes)
    expect(() => normalizeSignature('abc')).toThrow(/unexpected format/)
    const account = await connectKaspire(mockKaspire({ getPublicKey: () => `02${publicKey}` }))
    expect(account.publicKey).toBe(publicKey)
  })

  it('explains Kaspire errors and how to install it', () => {
    expect(walletErrorMessage({ code: 4001, message: 'User rejected' })).toMatch(/rejected in Kaspire/)
    expect(walletErrorMessage({ code: 4100, message: 'locked' })).toMatch(/locked/)
    expect(missingWalletMessage()).toMatch(/kaspire\.kaslab\.space/)
  })
})
