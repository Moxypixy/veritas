# Cost-of-Living Pulse Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship a Kaspa **testnet-only** Cost-of-Living Pulse app: wallet login, one vote per wallet per month with a locked KAS deposit and on-chain commitment hash, off-chain encrypted answers + aggregates, and a plain-language chart vs official price series.

**Architecture:** Hybrid. Argent covenant enforces vote slot + deposit lock/release and stores only a commitment hash. Rust builds transactions and runs the indexer/API. React web app handles consent, form, chart, and wallet UX. Official stats are fetched by a scheduled worker into the API.

**Tech Stack:** Argent (local checkout) + Rust 1.94 / Cargo workspace; Axum API; React + TypeScript (Vite); Kaspa WASM / injected wallet for testnet-10; PostgreSQL for indexer; AES-256-GCM at rest.

## Global Constraints

- Network: **Kaspa testnet only** (testnet-10). No mainnet claims or endpoints in v1 config.
- Deposit amount: **10_000_000 sompi** (0.1 KAS) fixed.
- Unlock: reclaim allowed starting the **next UTC calendar month** after the vote’s `month_key`.
- Grace: **2 calendar months** after the unlock month begins. After grace ends, a public `abandon` path may spend the locked value to a fixed **unspendable burn script** (must not leave the UTXO unspendable forever).
- One vote per wallet per `month_key` (UTC `YYYY-MM`).
- On-chain stores **commitment hash only** — never plaintext survey fields.
- Public API/UI: **aggregates only**; suppress vote series when count &lt; **5**.
- Regions v1 (enable only after series verification): `euro-area`, `united-states`, `united-kingdom`, `japan`.
- Encryption: **AES-256-GCM** with server key from env; wallet signature auth for export/erase.
- Argent: **never invent syntax** — derive from local `argent` + working `.ag` examples (`Counter`, `Event`/`Ticket`); build with `./argentc` / `argentc` wrappers before accepting.
- Frontend independence: core Argent/Rust must not depend on React.
- Git workflow: **one git branch per task** below; **commit after each task**; **never `git push` unless the human explicitly asks**.
- Suggested branch names are listed on each task (`step-NN-…`). Create the branch from updated `master` (or prior merged step) before starting that task.
- Verification: use template `./check` / `check-win.ps1` plus frontend lint/typecheck/tests when those exist. Do not claim success without running the commands.

**Local tooling paths (this machine):**

- Argent language checkout: `C:\Users\linft\argent-demos\argent` (or sibling `../argent` after scaffold).
- Reference template / examples: under `C:\Users\linft\argent-demos\argent-playground\` (includes `argent-template` Counter/Event and Argent examples).
- Set `ARGENT_TEMPLATE_ARGENT_DIR` if the Argent checkout is not at `../argent` relative to this repo.

**Spec:** `docs/superpowers/specs/2026-08-08-cost-of-living-voting-design.md`

---

## File structure (target)

```
veritas/
  agents.md
  docs/superpowers/...
  ag/
    vote.ag                 # VoteSlot (+ any helper actors), app Pulse
  src/
    lib.rs                  # shared Rust: commitment encoding, fixtures
    bin/
      vote_local.rs         # local-runtime vote smoke
      reclaim_local.rs      # local-runtime reclaim smoke
  crates/
    veritas-api/            # Axum indexer/API
    veritas-inflation/      # official series worker (bin or lib+bin)
  web/                      # React + Vite + TS
  tests/                    # Rust integration tests where needed
  setup-win.ps1 / check-win.ps1 / argentc wrappers (from template)
  rust-toolchain.toml
  Cargo.toml                # workspace
```

---

### Task 1: Scaffold Argent+Rust workspace

**Branch:** `step-01-scaffold`

**Files:**
- Create: `Cargo.toml` (workspace), `rust-toolchain.toml`, `src/lib.rs`, `src/bin/counter.rs` (temporary smoke from template or rename later), `ag/counter.ag` (temporary), `setup-win.ps1`, `check-win.ps1`, argentc wrappers as in template
- Modify: `.gitignore` so tracked project scripts (`setup-win.ps1`, `check-win.ps1`, `argentc*`) are **not** ignored; keep `target/`, `build/`, `.env*`, `node_modules/` ignored; do **not** git-add nested `argent/` checkouts
- Preserve: `agents.md`, `docs/`

**Interfaces:**
- Consumes: local Argent checkout + argent-template layout
- Produces: `cargo test`, `cargo run --bin counter` (or equivalent), `./argentc build ag/counter.ag` working from this repo

- [ ] **Step 1: Create branch**

```bash
git checkout master
git pull   # only if tracking remote and human asked; otherwise skip pull
git checkout -b step-01-scaffold
```

- [ ] **Step 2: Copy scaffold from argent-template into this repo**

Copy the minimal template surface (not a nested git clone of the whole template repo into veritas):

- `rust-toolchain.toml` (Rust 1.94.x + clippy/rustfmt)
- Cargo package/workspace layout matching template dependencies (`argent`, `argent-runtime`, `kaspa-consensus-core` tag `v2.0.1`, etc.)
- `ag/counter.ag` exactly from the template Counter demo
- `src/bin/counter.rs` and supporting `src/lib.rs` as in template
- Windows setup/check scripts (`setup-win.ps1`, `check-win.ps1`) and `argentc` wrappers

Point Argent path at the existing checkout (env or path dep), e.g. `ARGENT_TEMPLATE_ARGENT_DIR=C:\Users\linft\argent-demos\argent`, **or** document that `../argent` must exist as a sibling clone.

Update `.gitignore`:

```gitignore
.cursor/
.vscode/
# keep helper scripts used only by agents out of git if desired:
# /scripts/agent-*
*.env
.env.*
node_modules/
target/
build/
dist/
# do NOT ignore setup-win.ps1 / check-win.ps1 / argentc
```

- [ ] **Step 3: Run setup and verify Counter smoke**

```powershell
.\setup-win.ps1
.\argentc build ag/counter.ag
cargo test --all-features
cargo run --bin counter
```

Expected: build succeeds; counter demo runs against local runtime; no network calls.

- [ ] **Step 4: Commit (do not push)**

```bash
git add Cargo.toml rust-toolchain.toml ag/counter.ag src .gitignore setup-win.ps1 check-win.ps1 argentc* Cargo.lock
git commit -m "$(cat <<'EOF'
Scaffold Veritas from Argent template with local Counter smoke.

EOF
)"
```

---

### Task 2: Canonical commitment encoding

**Branch:** `step-02-commitment-encoding`

**Files:**
- Create: `src/commitment.rs`
- Modify: `src/lib.rs` (`mod commitment; pub use …`)
- Test: unit tests inside `src/commitment.rs` (or `tests/commitment_golden.rs`)

**Interfaces:**
- Consumes: none from later tasks
- Produces:

```rust
pub const COMMITMENT_DOMAIN: &[u8] = b"veritas.colp.v1";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VoteAnswer {
    pub region_id: String,      // ASCII enum id, e.g. "euro-area"
    pub necessities_pct: u8,    // 0..=100
    pub employed: bool,
    pub duration_months: u32,
    pub month_key: String,      // "YYYY-MM" UTC
    pub wallet: String,         // Kaspa address string as provided by wallet
    pub salt: [u8; 16],
}

pub fn canonical_bytes(answer: &VoteAnswer) -> Result<Vec<u8>, CommitmentError>;
pub fn commitment_hash(answer: &VoteAnswer) -> Result<[u8; 32], CommitmentError>;
```

**Canonical byte layout (lock this; clients must match):**

1. `COMMITMENT_DOMAIN` length-prefixed: `u32` LE length + bytes  
2. `region_id`: `u32` LE length + UTF-8 bytes  
3. `necessities_pct`: `u8`  
4. `employed`: `u8` (`0` or `1`)  
5. `duration_months`: `u32` LE  
6. `month_key`: `u32` LE length + UTF-8 bytes  
7. `wallet`: `u32` LE length + UTF-8 bytes  
8. `salt`: 16 raw bytes  

Hash: **BLAKE2b-256** over `canonical_bytes` (same family as Argent examples using `blake2b`).

- [ ] **Step 1: Create branch from updated master (or merge Task 1 first)**

```bash
git checkout -b step-02-commitment-encoding
```

- [ ] **Step 2: Write failing golden test**

```rust
#[test]
fn golden_commitment_vector_1() {
    let answer = VoteAnswer {
        region_id: "euro-area".into(),
        necessities_pct: 42,
        employed: true,
        duration_months: 18,
        month_key: "2026-08".into(),
        wallet: "kaspatest:qq…".into(), // use a fixed fixture string in the real test
        salt: [7u8; 16],
    };
    let bytes = canonical_bytes(&answer).unwrap();
    assert_eq!(bytes.len() > COMMITMENT_DOMAIN.len(), true);
    let hash = commitment_hash(&answer).unwrap();
    // After implementing, pin the exact 32-byte hex in the test:
    // assert_eq!(hex::encode(hash), "<pinned>");
    assert_ne!(hash, [0u8; 32]);
}
```

First commit of the test may assert `commitment_hash` is not yet defined (compile fail) or call a stub that panics.

- [ ] **Step 3: Run test — expect FAIL**

```powershell
cargo test golden_commitment_vector_1 -- --nocapture
```

- [ ] **Step 4: Implement `src/commitment.rs`**

Validate: `necessities_pct <= 100`, `month_key` matches `^\d{4}-\d{2}$`, `region_id` non-empty ASCII, `wallet` non-empty. Reject otherwise with `CommitmentError`.

Pin the golden hex once the encoder is stable; add a second vector with `employed: false`.

- [ ] **Step 5: Run tests — expect PASS**

```powershell
cargo test commitment --all-features
```

- [ ] **Step 6: Commit (do not push)**

```bash
git commit -m "$(cat <<'EOF'
Add canonical vote commitment encoding with golden vectors.

EOF
)"
```

---

### Task 3: Argent vote + deposit covenant

**Branch:** `step-03-argent-vote-covenant`

**Files:**
- Create: `ag/vote.ag` (name may match app; keep one app entry)
- Modify/remove: retire `ag/counter.ag` from the default app path once vote builds (or keep Counter only as a separate example if useful)
- Test: local Argent build + Rust rejection/success paths in Task 4; this task ends when `argentc build` + `inspect` succeed and domain rules are documented in comments

**Interfaces:**
- Consumes: commitment is `byte[32]` supplied at vote time (covenant does not re-encode survey fields)
- Produces: Argent artifact under `build/` for actors that enforce:

**Domain rules (must hold):**

1. `vote`: locks **exactly** `10_000_000` sompi into the vote/deposit output; records `voter` pubkey/address binding as required by Argent patterns; records `month_key` (encode as `int` `YYYYMM` or fixed bytes — pick one and document); records `commitment: byte[32]`.
2. Reject second `vote` for the same voter + same `month_key` (model via actor state / lineage — follow Event/Ticket or registry patterns from local examples; do not invent ICC features).
3. `reclaim`: allowed only when current UTC month ≥ unlock month (next month after vote); returns deposit to voter; only voter may reclaim before abandon.
4. `abandon`: allowed only when current month ≥ unlock month + 2; spends value to fixed burn script / unspendable output pattern verified against current Argent/Kaspa examples.

**Reference examples to read before writing any `.ag`:**

- `…/argent-template/ag/counter.ag` — `state`, `actor`, `entry`, `require`, `become`, `blake2b`
- `…/argent-template/ag/event.ag` — multi-output `emits`, value constraints (`event.value == self.value + self.price`)
- Nearby Argent examples under `argent-demos/argent/examples/` for time/value patterns if present

- [ ] **Step 1: Create branch**

```bash
git checkout -b step-03-argent-vote-covenant
```

- [ ] **Step 2: Inspect working examples and draft `ag/vote.ag`**

Draft using **only** constructs seen in those files / compiler-accepted code. Document month encoding and burn output in comments (WHY).

Illustrative shape (adjust until compiler accepts — this is a starting sketch from Counter/Event patterns, not a guarantee):

```argent
import "std::core";

state VoteDepositState {
    pubkey voter;
    int month_yyyymm;
    byte[32] commitment;
}

actor VoteDeposit owns VoteDepositState {
    // entries: reclaim / abandon — exact signatures per verified Argent patterns
}

app Pulse {
    actor VoteDeposit;
}
```

If a separate factory/registry actor is required for one-vote-per-month, add it in the same app; keep the model readable.

- [ ] **Step 3: Build and inspect**

```powershell
.\argentc build ag/vote.ag
.\argentc inspect build/argent
```

Expected: success. On failure: fix from compiler diagnostics; do not add shims that hide misunderstanding.

- [ ] **Step 4: Commit (do not push)**

```bash
git commit -m "$(cat <<'EOF'
Add Argent vote deposit covenant for monthly commitments.

EOF
)"
```

---

### Task 4: Local-runtime vote and reclaim binaries

**Branch:** `step-04-local-runtime-bins`

**Files:**
- Create: `src/bin/vote_local.rs`, `src/bin/reclaim_local.rs`
- Modify: `src/lib.rs` helpers for loading artifact + building txs (mirror template `counter` patterns)
- Test: `tests/vote_local_rules.rs` or unit tests that drive local runtime

**Interfaces:**
- Consumes: `build/` artifact from Task 3; `commitment_hash` from Task 2 for fixture answers
- Produces: deterministic local demos:

```rust
// Conceptual CLI behaviour
// vote_local: create vote deposit for fixture wallet + month + commitment
// reclaim_local: reclaim after month advance fixture
```

- [ ] **Step 1: Create branch**

```bash
git checkout -b step-04-local-runtime-bins
```

- [ ] **Step 2: Write failing tests for rules**

Tests (local runtime, not network):

1. First vote for `(wallet, month)` succeeds.  
2. Second vote same `(wallet, month)` fails.  
3. Reclaim in vote month fails.  
4. Reclaim in following month succeeds.  
5. Abandon before grace end fails; after grace succeeds (fixture clock / month injection as supported by runtime).

Follow how `counter` binary constructs and applies txs in the template — copy that orchestration style.

- [ ] **Step 3: Run tests — expect FAIL**

```powershell
cargo test vote_local --all-features
```

- [ ] **Step 4: Implement bins + helpers; make tests pass**

```powershell
cargo test --all-features
cargo run --bin vote_local
cargo run --bin reclaim_local
.\check-win.ps1
```

- [ ] **Step 5: Commit (do not push)**

```bash
git commit -m "$(cat <<'EOF'
Add local-runtime vote and reclaim flows with rule tests.

EOF
)"
```

---

### Task 5: Indexer API — auth, submit, aggregates

**Branch:** `step-05-indexer-api`

**Files:**
- Create: `crates/veritas-api/` (Axum service), `crates/veritas-api/src/main.rs`, `auth.rs`, `submit.rs`, `aggregates.rs`, `db.rs`
- Modify: workspace `Cargo.toml` members
- Test: `crates/veritas-api/tests/*.rs` with test DB or sqlite if you choose sqlite for v1 local — **prefer SQLite for v1 local simplicity** unless Postgres is already required; document `DATABASE_URL`

**Interfaces:**
- Consumes: `veritas::commitment::{VoteAnswer, commitment_hash, canonical_bytes}`
- Produces HTTP JSON (illustrative):

```text
POST /v1/auth/challenge  -> { nonce, expires_at }
POST /v1/auth/verify     -> { session_token }  // wallet sig over challenge
POST /v1/votes            -> body: answer fields + salt + tx_id/proof refs
                           requires session; verifies commitment matches; verifies on-chain commitment (local stub trait in v1 tests)
GET  /v1/aggregates?region_id&month_key&employment=all|employed|unemployed
                           -> { necessities_avg_pct?, n, suppressed: bool }
```

Trust rules:

- Reject submit if `commitment_hash(answer) !=` declared/on-chain commitment.
- Do not store answer if chain verification fails.
- Aggregates never include wallet or raw answer.

- [ ] **Step 1: Create branch**

```bash
git checkout -b step-05-indexer-api
```

- [ ] **Step 2: Write failing API tests** (challenge verify, mismatch reject, aggregate suppression when `n < 5`)

- [ ] **Step 3: Implement minimal Axum API + DB schema**

Schema sketch:

```sql
CREATE TABLE answers (
  id INTEGER PRIMARY KEY,
  wallet TEXT NOT NULL,
  month_key TEXT NOT NULL,
  region_id TEXT NOT NULL,
  ciphertext BLOB NOT NULL,
  nonce BLOB NOT NULL,
  commitment BLOB NOT NULL,
  created_at TEXT NOT NULL,
  UNIQUE(wallet, month_key)
);

CREATE TABLE aggregate_bins (
  region_id TEXT NOT NULL,
  month_key TEXT NOT NULL,
  employment TEXT NOT NULL, -- all|employed|unemployed
  n INTEGER NOT NULL,
  necessities_sum INTEGER NOT NULL,
  PRIMARY KEY(region_id, month_key, employment)
);
```

Use a `ChainVerifier` trait:

```rust
#[async_trait]
pub trait ChainVerifier {
    async fn commitment_for_vote(&self, wallet: &str, month_key: &str) -> Result<Option<[u8;32]>, ApiError>;
}
```

In tests, use `FakeChainVerifier`.

- [ ] **Step 4: Run API tests — PASS**

```powershell
cargo test -p veritas-api --all-features
```

- [ ] **Step 5: Commit (do not push)**

```bash
git commit -m "$(cat <<'EOF'
Add indexer API for auth, vote submit, and aggregates.

EOF
)"
```

---

### Task 6: Encryption + GDPR export/erase

**Branch:** `step-06-gdpr-crypto`

**Files:**
- Create: `crates/veritas-api/src/crypto.rs`, `gdpr.rs`
- Modify: submit path to encrypt; add routes
- Test: encrypt/decrypt round-trip; erase removes row; export requires wallet session

**Interfaces:**

```text
GET  /v1/me/export  -> decrypted answers for session wallet (or encrypted package + decrypt client-side if chosen; v1: server decrypts after wallet auth)
DELETE /v1/me/answers -> deletes answers rows for wallet; aggregates remain as anonymous bins (not recomputed from personal rows)
```

Env: `VERITAS_DATA_KEY` = 32-byte key (base64). Refuse to start if missing in non-test profiles.

- [ ] **Step 1: Create branch `step-06-gdpr-crypto`**
- [ ] **Step 2: Failing tests for round-trip, erase, export auth**
- [ ] **Step 3: Implement AES-256-GCM helpers + routes**
- [ ] **Step 4: `cargo test -p veritas-api` PASS**
- [ ] **Step 5: Commit (do not push)**

```bash
git commit -m "$(cat <<'EOF'
Add answer encryption and GDPR export/erase endpoints.

EOF
)"
```

---

### Task 7: Inflation worker + plain-language series

**Branch:** `step-07-inflation-worker`

**Files:**
- Create: `crates/veritas-inflation/`, region map JSON, fetcher, normalizer
- Modify: API `GET /v1/chart?region_id=` combining official series + aggregates
- Test: parser fixtures; region disabled if a required series missing

**Interfaces:**

Plain-language keys (API → UI):

| key | meaning |
|-----|---------|
| `prices_overall` | How fast prices are rising overall |
| `prices_food` | Food prices |
| `prices_housing` | Housing costs |
| `prices_energy` | Energy / fuel prices |
| `voters_necessities` | What voters say they spend on necessities |

v1 sources: prefer OECD / Eurostat free JSON. Store last-success payload; on fetch failure, API sets `official_unavailable: true` while still returning vote aggregates when present.

- [ ] **Step 1: Create branch `step-07-inflation-worker`**
- [ ] **Step 2: Verify each region’s four series exist; commit `regions.json` with series IDs and labels**
- [ ] **Step 3: Failing tests with recorded HTTP fixtures (no live network in unit tests)**
- [ ] **Step 4: Implement worker bin + chart endpoint**
- [ ] **Step 5: Commit (do not push)**

```bash
git commit -m "$(cat <<'EOF'
Add inflation worker and plain-language chart API.

EOF
)"
```

---

### Task 8: Web app — consent + vote form

**Branch:** `step-08-web-consent-form`

**Files:**
- Create: `web/` Vite React TS app; pages/components for consent + form
- Modify: root README with `web` dev instructions (only if README exists or is created here)

**Interfaces:**
- Consumes: API types for submit; shared commitment encoding — either:
  - duplicate encoder in `web/src/commitment.ts` generated from the same golden vectors (must pass identical golden hex tests), or
  - small WASM/npm package built from Rust later (v1: TypeScript encoder with golden vectors copied from Rust tests)

UI requirements:

- Consent screen before first vote (on-chain vs off-chain, erase limits, deposit).
- Form: region, necessities %, employed, duration (dynamic label), deposit amount display `0.1 KAS`, confirmations.
- WCAG 2.2 AA: labels, keyboard, focus, error text.

- [ ] **Step 1: Create branch `step-08-web-consent-form`**
- [ ] **Step 2: Scaffold Vite React TS; add commitment golden test in Vitest**
- [ ] **Step 3: Implement consent + form (wallet button can be stub “Connect wallet” until Task 10)**
- [ ] **Step 4: `npm test` && `npm run lint` && `npm run build`**
- [ ] **Step 5: Commit (do not push)**

```bash
git commit -m "$(cat <<'EOF'
Add web consent flow and monthly vote form.

EOF
)"
```

---

### Task 9: Web chart UI

**Branch:** `step-09-web-chart`

**Files:**
- Create: `web/src/pages/ChartPage.tsx`, chart components
- Modify: routing/nav

**Interfaces:**
- Consumes: `GET /v1/chart`
- Shows plain-language series names only; toggle All / Employed / Unemployed; footnotes for dataset + last updated; “Not enough responses yet” when suppressed.

- [ ] **Step 1: Create branch `step-09-web-chart`**
- [ ] **Step 2: Component tests for suppression copy and filter toggle**
- [ ] **Step 3: Implement chart page**
- [ ] **Step 4: lint/test/build PASS**
- [ ] **Step 5: Commit (do not push)**

```bash
git commit -m "$(cat <<'EOF'
Add plain-language cost-of-living chart page.

EOF
)"
```

---

### Task 10: Wallet login + tx handoff (testnet config)

**Branch:** `step-10-wallet-testnet`

**Files:**
- Create: `web/src/wallet/` (detect injected provider, challenge sign, tx presentation)
- Modify: API auth verify; env examples `.env.example` (no secrets)
- Document: testnet-10 endpoints; **no auto-push of signed tx without user approval UI**

**Interfaces:**

Flow:

1. Request challenge → wallet signs → session  
2. Build commitment locally  
3. Construct covenant tx (Rust helper endpoint or client-side builder — prefer **server builds unsigned tx template**, wallet signs; never send seed/private keys)  
4. User approves in wallet  
5. Broadcast (testnet)  
6. POST `/v1/votes` with tx reference + full answer  

v1 wallet target: **injected Kaspa wallet (KasWare or equivalent)** + Kaspa WASM utilities as needed. If a connector is unavailable in CI, keep unit tests with mocks; manual testnet checklist in `docs/superpowers/manual-testnet-checklist.md`.

- [ ] **Step 1: Create branch `step-10-wallet-testnet`**
- [ ] **Step 2: Tests for challenge message format + reject bad signature**
- [ ] **Step 3: Implement connect/sign/submit wiring + clear error states**
- [ ] **Step 4: Manual checklist file; automated tests PASS**
- [ ] **Step 5: Commit (do not push)**

```bash
git commit -m "$(cat <<'EOF'
Wire Kaspa testnet wallet login and vote transaction handoff.

EOF
)"
```

---

### Task 11: End-to-end verification gate

**Branch:** `step-11-e2e-gate`

**Files:**
- Create: `docs/superpowers/manual-testnet-checklist.md` (if not in Task 10), CI script optional
- Modify: `check-win.ps1` to also run `web` lint/test when `web/` exists

**Success criteria mapping (from spec §13):**

| Criterion | How verified |
|-----------|----------------|
| Connect, consent, vote, deposit locked | Manual testnet checklist + local runtime demos |
| Second vote same month fails | Task 4 tests + manual |
| Next month reclaim | Task 4 tests + manual |
| Chart + employed filter | Task 7–9 tests |
| Erase removes off-chain; commitment remains | Task 6 tests |
| No bulky full answers on-chain | Covenant stores `byte[32]` only — review `ag/vote.ag` |

- [ ] **Step 1: Create branch `step-11-e2e-gate`**
- [ ] **Step 2: Extend check script; run full gate**

```powershell
.\check-win.ps1
cd web; npm test; npm run lint; npm run build
```

- [ ] **Step 3: Fill checklist results in the doc (what was actually run)**
- [ ] **Step 4: Commit (do not push)**

```bash
git commit -m "$(cat <<'EOF'
Add end-to-end verification gate and testnet checklist.

EOF
)"
```

---

## Merge / branch discipline (human + agent)

1. Start each task on its `step-NN-…` branch.  
2. Finish task → commit locally → **stop and show the human** the commit summary.  
3. Human merges to `master` (or asks agent to merge locally).  
4. **Never `git push` unless the human explicitly says to push.**  
5. Next task branches from updated `master`.

---

## Spec coverage checklist (plan self-review)

| Spec area | Task |
|-----------|------|
| Wallet login | 10 |
| Monthly survey fields | 2, 5, 8 |
| One vote/month + deposit | 3, 4 |
| Commitment on-chain only | 2, 3, 5 |
| Encrypted off-chain + aggregates | 5, 6 |
| GDPR export/erase | 6 |
| k-anonymity threshold 5 | 5, 9 |
| Consent UX | 8 |
| Chart + plain language | 7, 9 |
| Official series regions | 7 |
| Testnet only | Global + 10 |
| Argent/Rust separation | 1–4, agents.md |
| Errors table | 5, 8, 10 |
| Local build/inspect/check | 1, 3, 4, 11 |

Open points from spec §14 resolved in Global Constraints (deposit, grace/abandon, wallets, regions, encryption). Argent actor shape finalized in Task 3 against the live compiler.
