# Task 10 report — Wallet login + transaction handoff

## Status

Implemented the browser-side KasWare-compatible Testnet-10 connection and challenge-signing path, local commitment review, API challenge message delivery, public-key plumbing, and testnet-only configuration. The production API remains fail-closed for both wallet-signature and chain verification because no verified server-side KasWare message-verification format or live covenant transaction builder exists in this repository.

## Verification

- `cargo test -p veritas-api --all-features` — 12 API integration tests passed, including challenge message shape and forged-signature rejection through the mock verifier.
- `cargo fmt --check` and `cargo clippy --all-targets --all-features -- -D warnings` — passed.
- `web: npm test`, `npm run lint`, and `npm run build` — 9 tests passed; typecheck and production build passed.
- `.\check-win.ps1` — passed.

## Safety and integration notes

- The connector checks/switches to KasWare's documented `kaspa_testnet_10` network before requesting the login signature and sends no private material to the API.
- The UI presents the intended Testnet-10 deposit and commitment and disables signing/broadcast until the server can produce a verified unsigned covenant transaction template.
- Current KasWare documentation supplies browser-side `verifyMessage`, but it does not document a server-side message-preimage/verification contract that can be implemented here without guessing. The production `UnconfiguredAuthVerifier` therefore rejects, while test mocks cover accepted/rejected API wiring.
- No Testnet-10 wallet, RPC endpoint, live chain verifier, or transaction builder was available for a manual end-to-end broadcast. The manual checklist records the required validation sequence.
