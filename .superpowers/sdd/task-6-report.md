# Task 6 Report: Encryption + GDPR export/erase

## Delivered

- Replaced the transitional answer JSON storage with AES-256-GCM ciphertext and
  a fresh 96-bit nonce per answer, using `VERITAS_DATA_KEY` as a base64-encoded
  32-byte key.
- Made the production binary reject startup when the data key is missing or
  invalid; API tests inject a fixed test key.
- Added authenticated `GET /v1/me/export` for decrypted answers belonging only
  to the session wallet and `DELETE /v1/me/answers` to erase that wallet's
  personal rows without changing anonymous aggregate bins.
- Documented the required environment variable and GDPR endpoint behaviour.

## Tests

- Wrote the new round-trip encryption, export-authentication, export-decryption,
  and erase/aggregate-preservation tests before the implementation; the initial
  run failed because `DataKey` and the new state constructor did not yet exist.
- `cargo test -p veritas-api` passed: 8 integration tests.
- `cargo fmt --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`,
  `cargo test --workspace --all-features`, `.\check-win.ps1`, and `git diff --check` passed.
- A direct binary invocation with `DATABASE_URL=sqlite::memory:` and no
  `VERITAS_DATA_KEY` exited with `VERITAS_DATA_KEY must be set outside tests`.

## Security notes

- AES-GCM authentication rejects malformed nonce, ciphertext, and tag data
  without returning plaintext.
- Export and erase resolve the wallet exclusively from the existing, unexpired
  bearer session; request-supplied wallet identifiers are not accepted.
- Erasure is intentionally limited to personal `answers` rows. Existing
  aggregate bins remain anonymized historical measurements and are not
  recomputed.
