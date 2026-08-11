# Kastle Testnet-10 wallet connector

**Date:** 2026-08-11  
**Status:** Approved for planning  
**Related:** Cost-of-Living Pulse; Task 10 wallet login

## 1. Goal

Replace the KasWare-primary browser connector with **Kastle-only** so Veritas can be tested on phone and desktop with **Kaspa Testnet-10** faucet funds.

## 2. Why Kastle

| Wallet | Testnet-10 | Mobile dApp connect | Onboarding friction |
|--------|------------|---------------------|---------------------|
| Kastle | Documented `switchNetwork("testnet-10")` | Extension + mobile (`window.kastle`) | Lowest for this app |
| KasWare | Yes (`kaspa_testnet_10`) | Extension only | High on mobile |
| Kaspium | Unclear / custom node | No documented inject for web dApps | High (manual sign) |

Kastle is the v1 wallet. KasWare and Kaspium are out of scope for this change.

## 3. Architecture

Keep the existing auth boundary:

1. Browser detects `window.kastle`.
2. User connects → app forces Testnet-10.
3. API issues a login challenge for the wallet address.
4. Kastle signs the challenge message; public key is sent with the signature.
5. API verifies (when a verified verifier exists) and returns a session token.

No private keys or seed phrases leave the wallet. The production API remains fail-closed for signature verification until a Kastle message-verification contract is proven — same posture as the KasWare path.

Vote transaction construction and broadcast remain unavailable until a verified unsigned covenant template exists. This design only swaps the wallet connector and UX for login (and documents the later Kastle signing methods).

## 4. Connector contract

New module: `web/src/wallet/kastle.ts` (replaces KasWare as the primary path).

| Step | Kastle API | App behaviour |
|------|------------|---------------|
| Detect | `window.kastle` | If missing, show install guidance |
| Connect | `kastle.connect()` | Require success before continuing |
| Account | `kastle.getAccount()` → `{ address, publicKey }` | Use `address` as wallet id |
| Network | `getNetwork` / `switchNetwork("testnet-10")` | Reject if not `testnet-10` after switch |
| Login | `kastle.signMessage(challenge.message)` | Same challenge API as today |
| Later tx (not in this change) | `signTx` / `signAndBroadcastTx` | Only after covenant template exists |

### Network id mapping

- Kastle network id: `testnet-10`
- Existing app/env may still use `kaspa_testnet_10` for operator config
- The adapter treats Kastle’s `testnet-10` as the required wallet network; do not accept mainnet

## 5. UX

- Connect button: **Connect Kastle (Testnet-10)**
- Missing wallet: tell the user to install Kastle (extension or mobile) and reopen where `window.kastle` is injected
- Wrong network: prompt switch to Testnet-10; fail clearly if still wrong
- After connect: show the `kaspatest:…` address and a short faucet hint for [Testnet-10 faucet](https://faucet-tn10.kaspanet.io/)
- Rejection / cancel: keep clear copy that no signature or transaction was submitted

## 6. Files to change

- `web/src/wallet/kastle.ts` — new adapter
- `web/src/wallet/kastle.test.ts` — unit tests (network id, rejection copy, not-on-testnet path)
- `web/src/App.tsx` — wire Kastle connect + messages
- `web/src/wallet/api.ts` — import challenge/session types from the Kastle module
- Delete the KasWare primary connector (`web/src/wallet/kasware.ts` and `kasware.test.ts`); do not leave a dual-wallet path
- `docs/superpowers/manual-testnet-checklist.md` — Kastle + faucet steps
- Note in voting design open point: wallet connector set = **Kastle / Testnet-10**

## 7. Out of scope

- Server-side Kastle message verification implementation (still fail-closed until verified)
- Live covenant transaction build / broadcast
- Dual support for KasWare or Kaspium
- Mainnet configuration or claims

## 8. Testing & done criteria

**Automated**

- Unit tests for `testnet-10` constant, friendly reject messaging, and forced-network failure
- Existing web lint/typecheck/tests and API tests still pass

**Manual (checklist)**

1. Install Kastle; switch to Testnet-10
2. Copy `kaspatest:…` address; fund via TN10 faucet
3. Open Vote page → Connect Kastle → approve connect and network switch if prompted
4. Approve login message signature; confirm session shows the same address
5. Reject once; confirm no session / no tx submitted messaging

**Done when**

- With Kastle on Testnet-10, connect → network check → challenge sign → session (or clear errors) works end-to-end against the existing auth API wiring
- Without Kastle, install guidance is shown
- Checklist and UI no longer instruct users to use KasWare as the primary wallet
