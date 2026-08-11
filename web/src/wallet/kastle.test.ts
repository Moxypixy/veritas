import { describe, expect, it, vi } from 'vitest'

import {
  TESTNET_10,
  connectAndAuthenticate,
  missingWalletMessage,
  walletErrorMessage,
  type KastleProvider,
} from './kastle'

function mockProvider(overrides: Partial<KastleProvider> = {}): KastleProvider {
  return {
    connect: vi.fn(async () => true),
    getAccount: vi.fn(async () => ({
      address: 'kaspatest:qqveritasgoldenwallet1',
      publicKey: '11'.repeat(32),
    })),
    getNetwork: vi.fn(async () => TESTNET_10),
    switchNetwork: vi.fn(async () => TESTNET_10),
    signMessage: vi.fn(async () => 'sig'),
    ...overrides,
  }
}

describe('Kastle testnet connector', () => {
  it('uses the documented Kastle testnet-10 network identifier', () => {
    expect(TESTNET_10).toBe('testnet-10')
  })

  it('turns a rejected wallet request into clear user-facing copy', () => {
    expect(walletErrorMessage(new Error('User rejected the request.'))).toBe(
      'Wallet approval was rejected. No signature or transaction was submitted.',
    )
  })

  it('explains how to install Kastle when the provider is missing', () => {
    expect(missingWalletMessage()).toMatch(/Install Kastle/i)
    expect(missingWalletMessage()).toMatch(/window\.kastle/i)
  })

  it('rejects when the wallet remains off testnet-10 after switch', async () => {
    const provider = mockProvider({
      getNetwork: vi
        .fn()
        .mockResolvedValueOnce('mainnet')
        .mockResolvedValueOnce('mainnet'),
      switchNetwork: vi.fn(async () => 'mainnet'),
    })
    const api = {
      createChallenge: vi.fn(),
      verifyChallenge: vi.fn(),
    }
    await expect(connectAndAuthenticate(provider, api)).rejects.toThrow(/Testnet-10/i)
    expect(api.createChallenge).not.toHaveBeenCalled()
  })
})
