import { describe, expect, it } from 'vitest'

import { TESTNET_10, walletErrorMessage } from './kasware'

describe('KasWare testnet connector', () => {
  it('uses the documented testnet-10 network identifier', () => {
    expect(TESTNET_10).toBe('kaspa_testnet_10')
  })

  it('turns a rejected wallet request into clear user-facing copy', () => {
    expect(walletErrorMessage(new Error('User rejected the request.'))).toBe(
      'Wallet approval was rejected. No signature or transaction was submitted.',
    )
  })
})
