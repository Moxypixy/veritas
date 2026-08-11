# Task 7 Report — Inflation worker + plain-language series

## Delivered

- Added `veritas-inflation`, which reads the checked-in OECD region map, fetches
  monthly price-growth series, normalizes recorded CSV responses, and stores the
  last successful official data in SQLite.
- Added verified OECD CPI/HICP series for the euro area, United States, United
  Kingdom, and Japan. Regions missing a required plain-language series are
  skipped; a failed refresh preserves prior rows and marks official data
  unavailable.
- Added `GET /v1/chart?region_id=&employment=all|employed|unemployed`, returning
  plain-language official series plus the suppression-safe voter series.

## Verification

- `cargo test -p veritas-inflation` — 4 integration tests passed; all HTTP data
  used by unit tests is recorded in `tests/fixtures/oecd-prices.csv`.
- `cargo test -p veritas-api --test api chart_` — 2 chart API tests passed.
- `cargo clippy -p veritas-inflation -p veritas-api --all-targets --all-features -- -D warnings`
  and `cargo fmt --check` passed.
- A live worker smoke test updated all four configured regions from the OECD
  endpoint using a disposable SQLite database.

## Notes

- The chart deliberately omits official lines when the worker records a refresh
  failure, while retaining eligible voter aggregate points.
- The worker is manual/scheduler-ready; deployment scheduling is outside this task.

## Review follow-up

- A skipped region now persists `official_unavailable: true`, so stale official
  rows cannot be returned after configuration loses a required series.
- Added a fixture-backed regression that refreshes a region, removes
  `prices_energy`, refreshes to `Skipped`, and verifies the chart omits official
  lines while retaining the unavailable signal.
