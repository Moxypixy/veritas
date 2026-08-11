# OECD regions expansion + local insecure wallet auth

**Date:** 2026-08-11  
**Status:** Approved for planning  
**Related:** Cost-of-Living Pulse; Kastle Testnet-10 connector

## 1. Goal

1. Expand Veritas voting/chart regions from the fixed four to **every OECD CPI area that verifies** for the required official series.
2. Unblock local end-to-end UI testing with Kastle by adding an **explicitly gated insecure auth verifier** — without weakening the production default fail-closed posture.
3. Put the **chart and full vote form on one page** (chart on top, form underneath), removing the confusing Vote/Chart split.

## 2. Decisions locked

| Topic | Choice |
|-------|--------|
| Region source | Existing OECD SDMX CPI/HICP feed only |
| Region scope | All OECD areas that verify for required series (not continents, not invented world aggregates) |
| Wallet auth for local test | Option A: `VERITAS_ALLOW_INSECURE_AUTH=1` accepts non-empty Kastle signature + public key |
| Dev-login button | No — Kastle `connect` / `signMessage` still required |
| Real Schnorr/Kastle crypto verify | Out of scope (later) |
| Covenant tx broadcast | Out of scope (still unavailable) |
| Page layout | Single page: chart on top, full vote form underneath; remove Vote/Chart split nav |

## 3. Architecture

Three related changes on the current stack:

### 3.1 Regions

- Keep `crates/veritas-inflation` + `regions.json` as the official-series config.
- Add a verification path (script and/or worker dry-run) that probes OECD for candidate area codes and enables a region only when **all four** required series resolve with usable points:
  - `prices_overall`
  - `prices_food`
  - `prices_housing`
  - `prices_energy`
- Prefer monthly series; if only quarterly data exists but still satisfies the chart contract, document the cadence and allow.
- Do **not** invent continent aggregates (“Africa”, “Asia”, “world”). Eurostat/OECD do not provide a single official world inflation series suitable for this product.
- Ship an updated `regions.json` containing every passing area (`id` e.g. `australia`, UI label e.g. `Australia`).
- **Single source of truth:** web vote form and chart dropdowns consume the same region list derived from that config (shared JSON import or a generated module). Remove hard-coded four-region arrays in `App.tsx` / `ChartPage.tsx`.
- Chart behaviour unchanged: if official data is missing after ingest, set `official_unavailable` rather than fabricating lines.

### 3.2 Local insecure wallet auth

- Production default: keep `UnconfiguredAuthVerifier` (rejects / unavailable) so unsigned wallet strings cannot become sessions.
- When process env `VERITAS_ALLOW_INSECURE_AUTH=1` at API start: install `InsecureAcceptingAuthVerifier` that returns `true` only if `signature` and `public_key` are non-empty after trim.
- Existing challenge → verify → session flow stays: nonce binding, wallet match, expiry, consume-once.
- Emit a clear **startup warning** when the insecure flag is set.
- Document in `.env.example` and the manual testnet checklist: local testing only; never enable on a public deployment.
- Frontend remains Kastle-only Testnet-10 (`window.kastle`); no dual KasWare path; no Dev-login bypass.

```text
Browser (Kastle) --signMessage--> API /v1/auth/verify
                                      |
                    flag unset --> UnconfiguredAuthVerifier (fail-closed)
                    flag set   --> InsecureAcceptingAuthVerifier (non-empty sig+pubkey)
```

### 3.3 Single-page chart + vote UI

- Remove the Vote / Chart primary nav split (`/` vs `/chart`). One main page serves both.
- Vertical order: **chart section first**, **full vote form underneath** (consent → survey → wallet → review).
- Chart’s employed / unemployed / all control remains a **chart aggregate filter**, not part of the vote form’s employment question. Keep labels clear so the two are not confused.
- Region lists for chart filters and the vote form come from the same OECD-verified source of truth. Optional later polish: syncing the selected region between chart and form is nice-to-have, not required for v1 of this change.
- Prefer composing existing `ChartContent` / chart loaders into `App` rather than maintaining two competing page shells. Keep `/chart` as a redirect to `/` or drop it.
- Accessibility: one document landmark structure; chart and form each keep their headings; keyboard order follows visual order (chart → form).

## 4. Files (expected)

| Area | Likely touch |
|------|----------------|
| Regions config | `crates/veritas-inflation/regions.json` |
| Verification tooling | inflation crate and/or a small discover/verify helper |
| Web UI | `web/src/App.tsx`, `web/src/pages/ChartPage.tsx` (compose into App), shared regions module, `web/src/style.css` |
| Auth | `crates/veritas-api/src/main.rs`, auth tests, `.env.example` |
| Docs | `docs/superpowers/manual-testnet-checklist.md`; design open-point / plan region wording |

## 5. Testing

**Automated**

- Auth: flag unset → production path still unavailable/rejects; flag set → non-empty sig+pubkey creates session; empty signature fails.
- Regions: verification output records pass/fail; only passers in shipped `regions.json`; web tests assert shared list drives both dropdowns.

**Manual**

1. Start API with `VERITAS_ALLOW_INSECURE_AUTH=1`, SQLite, data key, `VERITAS_KASPA_NETWORK=kaspa_testnet_10`.
2. Open web (Vite proxy to API); Connect Kastle (Testnet-10); approve connect + message sign.
3. Confirm session shows the same `kaspatest:…` address.
4. Confirm vote/chart region lists include expanded OECD areas (not only the original four).
5. Confirm one page shows chart above and the full vote form below (no separate Chart nav required).
6. Restart API without the flag; confirm login fails closed again.

## 6. Out of scope

- Real Kastle/Kaspa message signature cryptography for production auth
- Live covenant transaction construction / broadcast
- Continent-level or “world” official series
- Non-OECD data providers for v1 of this change

## 7. Done criteria

- Expanded verified OECD regions appear in vote and chart UIs from one source of truth.
- Local insecure auth works behind `VERITAS_ALLOW_INSECURE_AUTH=1` with real Kastle signing.
- Production default remains fail-closed.
- Docs warn clearly that the insecure flag is local-only.
- Chart and full vote form live on one page (chart above, form below); split Vote/Chart navigation is gone.
