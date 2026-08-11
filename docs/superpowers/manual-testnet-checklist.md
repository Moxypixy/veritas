# Testnet-10 wallet handoff checklist

This application is testnet-only. Do not use a mainnet wallet, seed phrase, or private key.

## Before testing

1. Set process environment variables from `.env.example` (Vite may read a local ignored `.env`; the Rust API launcher must receive exported environment variables) and set `VERITAS_KASPA_NETWORK=kaspa_testnet_10`.
2. Configure `VERITAS_KASPA_RPC_URL` with an operator-controlled Testnet-10 RPC endpoint.
3. Start the API with a separately generated `VERITAS_DATA_KEY`; do not put it in the browser or commit it.
4. Install **Kastle**, fund a **Testnet-10** account via `https://faucet-tn10.kaspanet.io/` using the `kaspatest:…` address, and confirm Kastle reports network `testnet-10`. (Operator env uses `VERITAS_KASPA_NETWORK=kaspa_testnet_10`; the wallet network id is Kastle’s `testnet-10`.)

## Wallet login

1. Open the Vote page and select **Connect Kastle (Testnet-10)**.
2. Approve the Kastle connection prompt and, if requested, its Testnet-10 network switch.
3. Inspect the login message: it must name `Veritas`, the displayed wallet, a nonce, and an expiry.
4. Approve the message signature only after inspecting it. Reject once and confirm the page says no signature or transaction was submitted.
5. With a valid verifier configured, confirm a session is created for the same wallet; attempt a modified signature and confirm the API returns `401 wallet signature is invalid`.

## Transaction handoff

1. Complete consent and the survey, then select **Review vote**.
2. Confirm the review shows Testnet-10, a 0.1 KAS / 10,000,000 sompi deposit, and a commitment hash—not plaintext answers.
3. Before signing, verify the eventual server template has the covenant destination and expected amount; do not approve a template with a different network, destination, or amount.
4. Confirm the wallet is the only signer and no seed phrase/private key is requested or sent to the API.
5. Reject the wallet signing dialog once; confirm no broadcast or `POST /v1/votes` occurs.
6. After explicit wallet approval, broadcast the signed transaction, wait for the configured chain verifier, then submit the tx reference and full answer.

## Current integration limit

The repository currently has no live Testnet-10 covenant transaction builder or server-side Kastle message-verification protocol implementation. The UI therefore stops at review and the production API rejects wallet authentication/chain verification rather than accepting an unverified signature or silently broadcasting (fail-closed). Use the automated mock tests for the interface until those verified integrations are supplied.
