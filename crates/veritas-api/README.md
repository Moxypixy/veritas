# Veritas API

Set `DATABASE_URL` to a SQLite URL before starting the service:

```powershell
$env:DATABASE_URL = "sqlite://veritas.db"
cargo run -p veritas-api
```

The binary starts the HTTP listener on `127.0.0.1:3000`. It deliberately
rejects wallet authentication and chain verification until Task 10 adds the
testnet wallet and verifier integrations. API tests inject fake verifiers.

Task 5 stores the private answer JSON in the `ciphertext`-shaped column only
as a migration placeholder. Task 6 replaces it with AES-256-GCM encryption;
the public aggregate endpoints never return the payload.
