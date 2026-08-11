# Testnet-10 wallet handoff checklist

This application is testnet-only. Do not use a mainnet wallet, seed phrase, or private key.

## Before testing

1. Set process environment variables from `.env.example` (Vite may read a local ignored `.env`; the Rust API launcher must receive exported environment variables) and set `VERITAS_KASPA_NETWORK=kaspa_testnet_10`.
2. Configure `VERITAS_KASPA_RPC_URL` with an operator-controlled Testnet-10 RPC endpoint.
3. Start the API with a separately generated `VERITAS_DATA_KEY`; do not put it in the browser or commit it.
4. Install **Kastle**, fund a **Testnet-10** account via `https://faucet-tn10.kaspanet.io/` using the `kaspatest:…` address, and confirm Kastle reports network `testnet-10`. (Operator env uses `VERITAS_KASPA_NETWORK=kaspa_testnet_10`; the wallet network id is Kastle’s `testnet-10`.)
5. For local Kastle login testing only, set `VERITAS_ALLOW_INSECURE_AUTH=1` on the API process. **Never** enable this on a public deployment.

## UI layout and regions

1. Confirm one page shows the chart on top and the full vote form below (no separate Vote/Chart navigation).
2. Confirm region dropdowns list expanded OECD areas (from shared `regions.json`), not only the original four.

## Wallet login

1. Open the Vote page and select **Connect Kastle (Testnet-10)**.
2. Approve the Kastle connection prompt and, if requested, its Testnet-10 network switch.
3. Inspect the login message: it must name `Veritas`, the displayed wallet, a nonce, and an expiry.
4. Approve the message signature only after inspecting it. Reject once and confirm the page says no signature or transaction was submitted.
5. With a valid verifier configured, confirm a session is created for the same wallet; attempt a modified signature and confirm the API returns `401 wallet signature is invalid`.
6. Restart the API without `VERITAS_ALLOW_INSECURE_AUTH`; confirm **Connect Kastle** still fails closed at `/v1/auth/verify` (no session created).

## Transaction handoff

1. Complete consent and the survey, then select **Review vote**.
2. Confirm the review shows Testnet-10, a 0.1 KAS / 10,000,000 sompi deposit, and a commitment hash—not plaintext answers.
3. Before signing, verify the eventual server template has the covenant destination and expected amount; do not approve a template with a different network, destination, or amount.
4. Confirm the wallet is the only signer and no seed phrase/private key is requested or sent to the API.
5. Reject the wallet signing dialog once; confirm no broadcast or `POST /v1/votes` occurs.
6. After explicit wallet approval, broadcast the signed transaction, wait for the configured chain verifier, then submit the tx reference and full answer.

## Current integration limit

The repository currently has no live Testnet-10 covenant transaction builder or server-side Kastle message-verification protocol implementation. The UI therefore stops at review and the production API rejects wallet authentication/chain verification rather than accepting an unverified signature or silently broadcasting (fail-closed). Use the automated mock tests for the interface until those verified integrations are supplied.

## Verification results — 2026-08-11

### Automated gate

| Command | Outcome | Evidence |
| --- | --- | --- |
| `.\check-win.ps1` | PASS | Rust formatting, workspace check, tests, Clippy with warnings denied, Counter local-runtime smoke, web lint, web tests, and `git diff --check` completed successfully. The local rule suite passed 5 tests, including duplicate-vote rejection and next-month reclaim. Web tests passed 11 tests in 3 files. |
| `cd web; npm test; npm run lint; npm run build` | PASS | 11 Vitest tests in 3 files passed; TypeScript lint passed; Vite production build completed successfully. |

### Manual Testnet-10 wallet results

Not run / pending. This environment has no configured operator-controlled Testnet-10 RPC endpoint, funded Kastle Testnet-10 account, or verified live covenant transaction builder and server-side Kastle signature verifier. The fail-closed integration limit above remains in effect; no manual wallet, signature, transaction, broadcast, or confirmation result is claimed.

### Success criteria evidence

| Criterion | Result | Evidence |
| --- | --- | --- |
| Connect, consent, vote, and see the deposit locked | PENDING MANUAL | The Kastle Testnet-10 steps above were not run. Local runtime checks cover the vote/deposit path, but do not prove a live wallet handoff or broadcast. |
| A second vote in the same month fails | PASS (local runtime) / PENDING MANUAL | `tests/vote_local_rules.rs` rejection test passed through `.\check-win.ps1`. |
| Next-month deposit reclaim succeeds | PASS (local runtime) / PENDING MANUAL | `tests/vote_local_rules.rs` reclaim test passed through `.\check-win.ps1`. |
| Chart has plain-language series and employed filter | PASS (automated) | API and web chart tests passed through the verification gate; the web filter request test covers the employed/unemployed selection. |
| Erase removes off-chain data while the commitment remains | PASS (automated) / PENDING LIVE-CHAIN CONFIRMATION | API erase test passed through the verification gate. `ag/vote.ag` stores the commitment in `VoteDepositState`; no live Testnet-10 chain record was created to inspect. |
| No bulky full answers are stored on-chain | PASS (source review) | `ag/vote.ag` accepts and stores only `commitment: byte[32]` in `VoteDepositState`; it has no full-answer field. |
