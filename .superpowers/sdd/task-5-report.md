# Task 5 Report: Indexer API — auth, submit, aggregates

## Delivered

- Added the `veritas-api` Axum crate with SQLite storage selected by
  `DATABASE_URL`, challenge/session auth routes, vote submission, and aggregate
  reads.
- Submission binds the answer to the authenticated wallet, requires the API's
  observed UTC month, checks the canonical commitment against both the request
  and `ChainVerifier`, and atomically rejects a duplicate `(wallet, month)`.
- Aggregate responses expose only count, suppression state, and average;
  results with fewer than five responses suppress the average.
- The binary safely rejects live auth and chain verification until Task 10
  supplies the wallet and Kaspa integrations. Tests use fake verifiers.

## Verification

The initial API test suite was written before the API implementation and
failed on the missing public API types. The completed checks passed:

```powershell
cargo test -p veritas-api --all-features
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
.\check-win.ps1
git diff --check
```

The workspace test run completed 11 tests: 3 commitment tests, 5 local-runtime
vote tests, and 3 API tests.

## Trust boundary and follow-up

- The indexer is intentionally trusted to impose one global ballot per wallet
  per UTC month; local Argent lacks both a keyed global wallet registry and an
  observed UTC clock.
- The `ciphertext` and `nonce` columns are present now, but Task 5 stores a
  transitional JSON payload in `ciphertext`. Task 6 must replace it with
  AES-256-GCM before any deployment that receives real answers.
- Rate limiting, the concrete Kaspa wallet signature format, and live chain
  verification remain future integration work.
