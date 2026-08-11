# Task 12 final review fixes

## Completed

- Added serialized environment coverage proving `VERITAS_ALLOW_INSECURE_AUTH`
  enables insecure authentication only when its value is exactly `1`.
- Added a Vitest check that the web region list matches the ordered `{id, label}`
  entries in `crates/veritas-inflation/regions.json`.
- Documented the region synchronization requirement in the manual checklist and
  clarified the `AuthVerifier` local-only environment override.

## Verification

- `cargo test -p veritas-api --all-features` — PASS (17 tests).
- `cd web; npm test` — PASS (14 tests).
- `cd web; npm run lint` — PASS.
- `cargo fmt --check` — PASS.
