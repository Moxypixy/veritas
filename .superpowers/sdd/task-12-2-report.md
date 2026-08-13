# Task 12.2 report: OECD regions

## Status

Completed. `regions.json` now requires a human-readable `label`, and the
`verify_oecd_regions` binary validates every required OECD series through the
same CSV request and parsing path as production ingestion.

## Verification result

The verifier checked 41 candidates. Five regions passed all four series:
Euro area, Austria, Belgium, Switzerland, and Chile. Australia and Canada
returned 404 for one required series. After the initial successful requests,
OECD returned HTTP 429 for the remaining 34 candidates, so those regions were
not shipped.

## Checks

- `cargo run -p veritas-inflation --bin verify_oecd_regions`
- `cargo test -p veritas-inflation --all-features`
- `cargo fmt --all -- --check`
- `cargo clippy -p veritas-inflation --all-targets --all-features -- -D warnings`

All offline checks passed. The network verification completed with documented
per-region failures and retained only the verified subset.

## Fix: preserve known-good regions on transient verification failures

- Restored the previously shipped `united-states`, `united-kingdom`, and
  `japan` configurations from `de62676`, preserving their original series
  keys. The published file now contains the required union of the four
  previously known-good regions and the four Task 12.2 PASS-verified regions.
- `verify_oecd_regions` retains the checked-in regions and adds new PASS
  candidates rather than rebuilding the file from only this run's PASSes.
  HTTP 429 and transient network errors retry up to three times with a
  one-second linear backoff, then report `INCONCLUSIVE`; they are not
  permanent missing-series failures. Requests are also spaced by one second.
- Re-ran the verifier: it completed in 504 seconds and confirmed many
  additional candidates. Switzerland and Chile received 429 responses during
  this run but remained in the published union because their prior PASSes are
  authoritative.

## Fix verification

- `cargo test -p veritas-inflation --all-features` — 7 tests passed (run again
  after the final test update).
- `cargo fmt --all -- --check` — passed before final formatting; source was
  then formatted with `cargo fmt --all`.
- `cargo clippy -p veritas-inflation --all-targets --all-features -- -D warnings`
  — passed.
