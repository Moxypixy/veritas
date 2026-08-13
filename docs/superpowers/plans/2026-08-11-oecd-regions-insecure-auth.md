# OECD Regions, Local Insecure Auth, and Single-Page UI Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Expand Veritas to every OECD CPI area that verifies for the required series, unlock local Kastle login behind `VERITAS_ALLOW_INSECURE_AUTH=1`, and put chart + full vote form on one page (chart above, form below).

**Architecture:** Keep fail-closed auth by default; swap in an insecure accepting verifier only when the env flag is `1`. Expand `regions.json` via an OECD probe that keeps only areas with all four required series. Web consumes one region list and renders a single page: chart section first, vote form second.

**Tech Stack:** Rust (veritas-api, veritas-inflation), React + TypeScript (Vite), Vitest, OECD SDMX CSV already used by the inflation worker.

## Global Constraints

- Network: Kaspa testnet only (`kaspa_testnet_10` / Kastle `testnet-10`).
- Auth flag: `VERITAS_ALLOW_INSECURE_AUTH=1` only; any other value or unset = fail-closed.
- Insecure verifier: accept only when both `signature` and `public_key` are non-empty after trim; still use existing challenge/session binding.
- Regions: OECD SDMX only; no continent or “world” aggregates.
- Required series keys: `prices_overall`, `prices_food`, `prices_housing`, `prices_energy`.
- UI: one page; chart on top; full vote form underneath; remove Vote/Chart split nav.
- Chart employment filter stays a chart aggregate filter (All / Employed / Unemployed), separate from the vote form employment radios.
- No private keys on the server; no Dev-login button; no real Schnorr verification in this plan; no covenant tx broadcast.
- Git: one branch for this plan work; commit after each task; never push unless the human asks.
- Prefer TDD: failing test → implement → pass → commit.
- Follow `AGENTS.md` verification for touched crates/web.

## File structure

| File | Responsibility |
|------|----------------|
| `crates/veritas-api/src/auth.rs` | `AuthVerifier` trait + `UnconfiguredAuthVerifier` + `InsecureAcceptingAuthVerifier` + `auth_verifier_from_env()` |
| `crates/veritas-api/src/main.rs` | Wire env-selected verifier; keep chain verifier unconfigured |
| `crates/veritas-api/tests/api.rs` | Tests for insecure vs fail-closed auth behaviour |
| `.env.example` | Document `VERITAS_ALLOW_INSECURE_AUTH` |
| `crates/veritas-inflation/regions.json` | Expanded verified OECD regions with `label` |
| `crates/veritas-inflation/src/lib.rs` | Parse optional `label`; helpers to build candidate series keys / filter enabled |
| `crates/veritas-inflation/src/bin/verify_oecd_regions.rs` (or `tests/verify_oecd.rs` ignored by default) | Probe OECD; print/write passers |
| `web/src/regions.ts` | Single source of `{ id, label }[]` for UI (derived from regions.json) |
| `web/src/App.tsx` | Single page: chart above, form below; no split nav |
| `web/src/pages/ChartPage.tsx` | Export chart panel components; drop standalone page shell / nav |
| `web/src/pages/ChartPage.test.tsx` / new App tests | Shared regions + single-page assertions |
| `docs/superpowers/manual-testnet-checklist.md` | Insecure auth + single-page + expanded regions |
| `docs/superpowers/specs/2026-08-08-cost-of-living-voting-design.md` | Update region open-point wording |

Suggested branch: `step-12-oecd-auth-single-page` from current `master`.

---

### Task 1: Env-gated insecure auth verifier

**Branch:** `step-12-oecd-auth-single-page` (create if missing)

**Files:**
- Modify: `crates/veritas-api/src/auth.rs`
- Modify: `crates/veritas-api/src/main.rs`
- Modify: `crates/veritas-api/src/lib.rs` (re-export if needed)
- Modify: `crates/veritas-api/tests/api.rs`
- Modify: `.env.example`
- Modify: `crates/veritas-api/README.md` (one paragraph on the flag)

**Interfaces:**
- Consumes: existing `AuthVerifier` trait
- Produces:
  - `pub struct UnconfiguredAuthVerifier`
  - `pub struct InsecureAcceptingAuthVerifier`
  - `pub fn auth_verifier_from_env() -> Arc<dyn AuthVerifier>`
  - `pub fn insecure_auth_enabled() -> bool` — true iff env equals `"1"`

- [ ] **Step 1: Write failing unit tests for the insecure verifier**

Add to `crates/veritas-api/src/auth.rs` under `#[cfg(test)]` (or a focused test module):

```rust
#[tokio::test]
async fn insecure_verifier_accepts_non_empty_signature_and_public_key() {
    let verifier = InsecureAcceptingAuthVerifier;
    assert!(verifier
        .verify("kaspatest:qq", "challenge", "sig", "pubkey")
        .await
        .unwrap());
}

#[tokio::test]
async fn insecure_verifier_rejects_empty_signature_or_public_key() {
    let verifier = InsecureAcceptingAuthVerifier;
    assert!(!verifier
        .verify("kaspatest:qq", "challenge", "  ", "pubkey")
        .await
        .unwrap());
    assert!(!verifier
        .verify("kaspatest:qq", "challenge", "sig", "")
        .await
        .unwrap());
}
```

- [ ] **Step 2: Run tests — expect FAIL (types missing)**

```powershell
cargo test -p veritas-api insecure_verifier --all-features
```

Expected: compile error — `InsecureAcceptingAuthVerifier` not found.

- [ ] **Step 3: Implement verifiers + env selection**

Move `UnconfiguredAuthVerifier` from `main.rs` into `auth.rs`. Add:

```rust
pub struct UnconfiguredAuthVerifier;

#[async_trait]
impl AuthVerifier for UnconfiguredAuthVerifier {
    async fn verify(
        &self,
        _wallet: &str,
        _challenge: &str,
        _signature: &str,
        _public_key: &str,
    ) -> Result<bool, ApiError> {
        Err(ApiError::unavailable(
            "wallet authentication is not configured",
        ))
    }
}

pub struct InsecureAcceptingAuthVerifier;

#[async_trait]
impl AuthVerifier for InsecureAcceptingAuthVerifier {
    async fn verify(
        &self,
        _wallet: &str,
        _challenge: &str,
        signature: &str,
        public_key: &str,
    ) -> Result<bool, ApiError> {
        Ok(!signature.trim().is_empty() && !public_key.trim().is_empty())
    }
}

pub fn insecure_auth_enabled() -> bool {
    std::env::var("VERITAS_ALLOW_INSECURE_AUTH").as_deref() == Ok("1")
}

pub fn auth_verifier_from_env() -> Arc<dyn AuthVerifier> {
    if insecure_auth_enabled() {
        eprintln!(
            "WARNING: VERITAS_ALLOW_INSECURE_AUTH=1 — wallet signatures are NOT cryptographically verified. Local testing only."
        );
        Arc::new(InsecureAcceptingAuthVerifier)
    } else {
        Arc::new(UnconfiguredAuthVerifier)
    }
}
```

In `main.rs`, replace `Arc::new(UnconfiguredAuthVerifier)` with `auth_verifier_from_env()` and remove the local struct.

- [ ] **Step 4: Add API integration coverage**

In `tests/api.rs`, add a test that builds an app with `InsecureAcceptingAuthVerifier` and confirms challenge→verify with non-empty signature yields a session, and empty signature returns 401:

```rust
#[tokio::test]
async fn insecure_verifier_creates_session_for_non_empty_signature() {
    let database = Database::connect("sqlite::memory:").await.unwrap();
    let clock = FixedClock::new(Utc.with_ymd_and_hms(2026, 8, 10, 12, 0, 0).unwrap());
    let app = router(AppState::new(
        database,
        DataKey::from_base64("AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=").unwrap(),
        Arc::new(FakeChainVerifier { commitment: None }),
        Arc::new(InsecureAcceptingAuthVerifier),
        Arc::new(clock),
    ));
    // challenge then verify with signature "any" and public_key "pk" → 200 + session_token
}
```

(Adapt `FakeChainVerifier` construction to match existing test helpers.)

- [ ] **Step 5: Run tests — expect PASS**

```powershell
cargo test -p veritas-api --all-features
```

Expected: all veritas-api tests pass.

- [ ] **Step 6: Document the flag**

Append to `.env.example`:

```env
# Local UI testing only. When set to 1, the API accepts any non-empty wallet
# signature + public key without cryptographic verification. Never enable on a
# public deployment. Unset or any other value keeps auth fail-closed.
# VERITAS_ALLOW_INSECURE_AUTH=1
```

Update `crates/veritas-api/README.md` with the same warning.

- [ ] **Step 7: Commit**

```powershell
git add crates/veritas-api .env.example
git commit -m "Add env-gated insecure wallet auth for local testing."
```

---

### Task 2: Expand and verify OECD regions

**Files:**
- Modify: `crates/veritas-inflation/regions.json`
- Modify: `crates/veritas-inflation/src/lib.rs`
- Create: `crates/veritas-inflation/src/bin/verify_oecd_regions.rs` (add `[[bin]]` in crate `Cargo.toml` if needed)
- Modify: `crates/veritas-inflation/tests/` as needed for parse/`label` tests

**Interfaces:**
- Consumes: existing OECD `base_url` + CSV parse path
- Produces:
  - Each region object includes `"label": "Australia"` (human UI string)
  - `regions.json` contains every OECD/`REF_AREA` candidate that returns usable points for all four required series
  - Binary `verify_oecd_regions` that fetches and prints pass/fail (used to refresh JSON)

**Candidate area codes (start list; verifier may drop any that fail):**

National CPI template `{AREA}.M.N.CPI.PA.{COICOP}.N.GY` with COICOP `_T`, `CP01`, `CP04`, `CP045`:  
`AUS`, `AUT`, `BEL`, `CAN`, `CHE`, `CHL`, `COL`, `CRI`, `CZE`, `DEU`, `DNK`, `ESP`, `EST`, `FIN`, `FRA`, `GBR`, `GRC`, `HUN`, `IRL`, `ISL`, `ISR`, `ITA`, `JPN`, `KOR`, `LTU`, `LUX`, `LVA`, `MEX`, `NLD`, `NOR`, `NZL`, `POL`, `PRT`, `SVK`, `SVN`, `SWE`, `TUR`, `USA`, and partner economies present in the same OECD dataset when they pass (e.g. `CHN`, `ZAF` if series resolve).

Keep existing euro-area HICP mapping (`EA20` + `HICP`) as today.

- [ ] **Step 1: Extend region JSON schema with `label` + failing parse test if needed**

Update deserialization so `label: String` is required on each region. Add a small test:

```rust
#[test]
fn region_requires_label() {
    let json = include_str!("../regions.json");
    let regions = parse_regions(json).unwrap();
    assert!(regions.iter().all(|r| !r.label.is_empty()));
}
```

- [ ] **Step 2: Implement verify binary**

Binary responsibilities:

1. Build candidate region configs (id slug from English name, `source_area`, measure `CPI` or `HICP`, four series keys).
2. For each series key, HTTP GET the existing OECD `base_url` pattern used by the worker (same Accept/CSV path as production ingest).
3. Parse with `parse_oecd_csv`; require at least one valid `YYYY-MM` point.
4. Print PASS/FAIL per region; write or stdout a `regions.json` fragment for passers only.
5. Network failures for a region → FAIL that region (do not ship it).

Run (network required):

```powershell
cargo run -p veritas-inflation --bin verify_oecd_regions
```

Expected: list of PASS regions; copy passers into `regions.json` with labels.

- [ ] **Step 3: Ship updated `regions.json`**

Replace contents with verified passers only. Keep source block. Ensure `region_is_enabled` remains true for every shipped region.

- [ ] **Step 4: Run offline tests**

```powershell
cargo test -p veritas-inflation --all-features
```

Expected: PASS (no network in unit tests).

- [ ] **Step 5: Commit**

```powershell
git add crates/veritas-inflation
git commit -m "Expand regions.json to verified OECD CPI areas."
```

---

### Task 3: Shared web regions + single-page chart-above-form UI

**Files:**
- Create: `web/src/regions.ts`
- Create: `web/src/regions.json` (copy of id/label list from inflation `regions.json`, kept in sync in this commit)
- Modify: `web/src/App.tsx`
- Modify: `web/src/pages/ChartPage.tsx`
- Modify: `web/src/pages/ChartPage.test.tsx`
- Create or modify: `web/src/App.test.tsx` (if practical) for “chart then form” structure
- Modify: `web/src/style.css` (only if needed for stacked sections)
- Keep: `web/vite.config.ts` proxy if already present for local `/v1`

**Interfaces:**
- Consumes: `{ id: string; label: string }[]` from `web/src/regions.ts`
- Produces: one page at `/` with chart section then vote form; `/chart` redirects to `/` or renders the same page

- [ ] **Step 1: Write failing tests for shared regions + page structure**

```ts
import { describe, expect, it } from 'vitest'
import { REGIONS } from './regions'

describe('shared regions', () => {
  it('exposes more than the original four OECD areas', () => {
    expect(REGIONS.length).toBeGreaterThan(4)
    expect(REGIONS.every((r) => r.id && r.label)).toBe(true)
  })
})
```

Add a DOM test (Testing Library if already available; otherwise lightweight `render` pattern used in `ChartPage.test.tsx`) asserting the main page includes chart heading/region control before the vote form legend “About your household costs”.

- [ ] **Step 2: Run web tests — expect FAIL**

```powershell
cd web; npm test
```

Expected: FAIL — `./regions` missing or page still split.

- [ ] **Step 3: Implement `regions.ts` + single page**

`web/src/regions.ts`:

```ts
import raw from './regions.json'

export type RegionOption = { id: string; label: string }

export const REGIONS: RegionOption[] = raw.regions.map((region: { id: string; label: string }) => ({
  id: region.id,
  label: region.label,
}))
```

`App.tsx` structure (conceptual):

```tsx
export default function App() {
  return (
    <main className="page">
      <header className="hero">…</header>
      <ChartPanel />   {/* filters + ChartDataSection; no Vote/Chart nav */}
      <VoteSection />  {/* consent + full form as today */}
    </main>
  )
}
```

- Remove `Navigation` Vote/Chart links.
- If `pathname === '/chart'`, `window.location.replace('/')` or render the same tree.
- Chart employment filter legend text: keep “Employment filter” so it is not confused with the form’s “Employment” fieldset.
- Import `REGIONS` from `./regions` in both chart panel and vote form.

- [ ] **Step 4: Run web lint/tests**

```powershell
cd web; npm test; npm run lint
```

Expected: PASS.

- [ ] **Step 5: Commit**

```powershell
git add web
git commit -m "Put chart above vote form on one page with shared OECD regions."
```

---

### Task 4: Docs + checklist alignment

**Files:**
- Modify: `docs/superpowers/manual-testnet-checklist.md`
- Modify: `docs/superpowers/specs/2026-08-08-cost-of-living-voting-design.md` (region open-point / success wording)
- Modify: `docs/superpowers/specs/2026-08-11-oecd-regions-insecure-auth-design.md` status → Implemented (optional one-liner)

- [ ] **Step 1: Update checklist**

Add:

1. Set `VERITAS_ALLOW_INSECURE_AUTH=1` for local login testing; warn never for public deploy.
2. Single page: chart on top, form below.
3. Region dropdown lists expanded OECD areas.
4. Without the flag, Connect Kastle still fails closed at verify.

- [ ] **Step 2: Update voting design open point**

Replace fixed four-region wording with: regions = all OECD areas verified for required series (see inflation `regions.json`).

- [ ] **Step 3: Commit**

```powershell
git add docs
git commit -m "Document local insecure auth, OECD regions, and single-page UI."
```

---

## Spec coverage checklist

| Spec requirement | Task |
|------------------|------|
| `VERITAS_ALLOW_INSECURE_AUTH=1` insecure accept | 1 |
| Default fail-closed | 1 |
| Startup warning + `.env.example` | 1 |
| OECD-only; no continents | 2 |
| Verify all four series before ship | 2 |
| `regions.json` + labels | 2 |
| Shared region list in web | 3 |
| Chart above, form below; no split nav | 3 |
| Chart filter ≠ form employment | 3 |
| Checklist / design docs | 4 |
| No real Schnorr / no tx broadcast | Global (out of scope) |

## Execution handoff

After this plan is accepted, implement task-by-task with review gates. Prefer Subagent-Driven Development unless the human chooses Inline Execution.
