# Veritas API

Set `DATABASE_URL` to a SQLite URL before starting the service:

```powershell
$env:DATABASE_URL = "sqlite://veritas.db"
$env:VERITAS_DATA_KEY = "<base64-encoded 32-byte key>"
cargo run -p veritas-api
```

The binary starts the HTTP listener on `127.0.0.1:3000`. It deliberately
rejects wallet authentication and chain verification until Task 10 adds the
testnet wallet and verifier integrations. API tests inject fake verifiers. It
refuses to start without a valid `VERITAS_DATA_KEY`; tests inject an explicit
key instead.

For local UI testing only, set `VERITAS_ALLOW_INSECURE_AUTH=1` to accept any
non-empty wallet signature and public key without cryptographic verification.
Never enable this flag on a public deployment; when unset or set to any other
value, wallet authentication remains fail-closed.

Answers are encrypted with AES-256-GCM at rest. `GET /v1/me/export` returns
the decrypted answers only for the wallet bound to the bearer session.
`DELETE /v1/me/answers` erases that wallet's personal answer rows; it does not
recompute anonymous aggregate bins.
