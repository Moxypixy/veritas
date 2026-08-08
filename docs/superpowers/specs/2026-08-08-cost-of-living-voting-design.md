# Cost-of-Living Pulse — Design Spec

**Date:** 2026-08-08  
**Project:** Veritas (Kaspa / Argent + Rust)  
**Network (v1):** Kaspa testnet only  
**Status:** Draft for implementation planning  

## 1. Purpose

A Kaspa testnet app where people report how much of their wage goes to necessities, compare that (as a crowd average) to official public price statistics, and do so with:

- Kaspa wallet login
- One vote per wallet per calendar month
- A small KAS deposit locked on vote and returned the following month if rules were followed
- Immutable answer commitments on-chain (without spamming the network with full payloads)
- Strong protection of individuals (GDPR-oriented): public surfaces show aggregates only

Working product name: **Cost-of-Living Pulse**.

## 2. Goals and non-goals

### Goals (v1)

- Wallet-authenticated monthly survey submission
- Fields: region, necessities % of wage (0–100), employed yes/no, duration in months (for both employed and unemployed)
- Fixed region list limited to places with free official statistics for overall prices plus food, housing, and energy (or closest equivalents)
- Chart: official series as lines, juxtaposed with voters’ monthly average necessities %; filter All / Employed / Unemployed
- Plain-language labels for every series (no unexplained CPI jargon in the primary UI)
- On-chain: deposit lock/release + one-vote-per-month + commitment hash
- Off-chain: encrypted full answers, aggregates API, erase/export for GDPR
- Testnet only

### Non-goals (v1)

- Mainnet
- Stronger sybil resistance than deposit + one address per month (e.g. proof-of-human)
- Per-wallet public vote feed
- Editable or deletable on-chain commitments
- Mobile-native apps
- Exact legal entity / DPO setup (product must still ship privacy UX and minimization)

## 3. Architecture (hybrid)

| Layer | Responsibility |
|--------|----------------|
| Kaspa wallet | Login challenge signature; approve deposit / release transactions |
| Argent covenant (L1) | Per-wallet per-month vote slot; deposit lock; release next month if rules held; store commitment hash only |
| Rust tx builders | Construct transactions against the Argent artifact (argent-template style), testnet endpoints |
| Indexer + API | Verify wallet proof + on-chain commitment; store encrypted answer; aggregates; erase/export |
| Inflation worker | Fetch official free series; map to plain-language chart payload |
| Web app | Consent, form, chart, deposit status |

### Immutability without spam

1. Client builds a canonical answer payload + random salt.
2. `commitment = hash(canonical_payload)`.
3. Covenant records `commitment`, month key, and locks deposit (small fixed testnet KAS).
4. Indexer accepts the full payload only if it hashes to the on-chain commitment and the wallet proof is valid.
5. Full JSON is not placed on the DAG.

### Trust boundary (must be stated in UI)

- **On-chain trust:** one-vote-per-month and deposit lock/release.
- **Indexer trust:** honest aggregation and encryption at rest. Chart averages are not consensus-critical.
- Users are told before voting what is immutable on Kaspa vs erasable on our servers.

## 4. Vote data model

### Submitted fields

| Field | Type | Notes |
|--------|------|--------|
| `region_id` | enum | Fixed list; only regions with required official series |
| `necessities_pct` | int 0–100 | Share of wage spent on necessities (housing, food, petrol combined) |
| `employed` | bool | |
| `duration_months` | int ≥ 0 | If employed: how long employed. If not: how long unemployed. Always required. |
| `month_key` | `YYYY-MM` | UTC calendar month of the vote |
| `wallet` | Kaspa address | From wallet login |
| `salt` | bytes | Random; included in commitment so answers are not trivially brute-forced from the hash |
| `commitment` | hash | `hash(canonical_encoding(...))` |

### On-chain record (minimal)

- Wallet / covenant lineage identity as required by Argent design
- `month_key`
- `commitment`
- Deposit amount and lock state
- No plaintext necessities %, employment, or region on-chain

### Canonical encoding

Implementation plan must define a single canonical byte encoding (field order, integer endianness, UTF-8 region id). Clients and indexer must share the same encoder; tests lock it.

## 5. Deposit and monthly rules

1. User connects Kaspa wallet and signs a login challenge.
2. User consents to privacy notice, then submits the form.
3. App builds commitment; user broadcasts covenant tx that:
   - Locks a **fixed small testnet KAS deposit**
   - Records commitment + month
   - Fails if this wallet already has a vote for `month_key`
4. Indexer verifies and stores encrypted answer; updates aggregates.
5. **Following calendar month:** user may claim deposit back if they cast exactly one valid vote for the prior month (rules followed).
6. **Grace period:** claims remain available for a defined window (default: 2 months after unlock month). After that, deposit is abandoned (exact handling in implementation plan; must not brick the covenant).

Spam cost is the locked deposit opportunity cost / capital lock, not large on-chain payloads.

## 6. Privacy and GDPR

### Principles

- Protect the individual; freeze the answer via on-chain commitment.
- Public API and UI never expose wallet ↔ answer linkage.
- Data minimization: no name, email, or phone required for v1 voting.

### Storage

| Location | Content | Erasure |
|----------|---------|---------|
| Kaspa L1 | Commitment hash, month, deposit state, wallet as required by chain | Not erasable |
| Indexer DB | Encrypted full answer; keys for aggregates | Erasable / exportable |

### User rights (off-chain)

- **Access / export:** user can decrypt/export their stored answers via wallet auth.
- **Erase:** delete encrypted payloads and identifying index rows from our DB.
- Aggregates are maintained as anonymous running statistics (updated at submit time). After erase they are not recomputed from personal rows and must never remain joinable to a wallet in the API.

### Chart anonymity

- Return aggregates only.
- Suppress series points when voter count &lt; **5** (configurable): show “Not enough responses yet”.

### Consent UX (before first vote)

Clear notice covering:

- What is stored on-chain vs off-chain
- That commitments cannot be removed from Kaspa
- That erase removes server-side readable/encrypted answers
- Deposit behaviour

## 7. Charts and UI

### Vote form

- Region dropdown (fixed list)
- Necessities % with helper text listing housing, food, petrol as included necessities
- Employed yes/no
- Duration months with dynamic label (employed vs unemployed)
- Deposit amount + privacy/immutability confirmation
- Submit

### Chart

- Region selector
- Line chart with plain-language series:
  - How fast prices are rising overall
  - Food prices
  - Housing costs
  - Energy / fuel prices
  - What voters say they spend on necessities (monthly average %)
- Toggle: All / Employed / Unemployed
- X-axis: months; Y-axis: percent
- Footnotes: dataset names, last updated, methodology link

### Out of UI scope (v1)

- Individual vote lists
- Wallet addresses on the chart

## 8. Official statistics

- Prefer free official sources (e.g. OECD, Eurostat, national statistics offices).
- Enable a region in the product only when overall + food + housing + energy (or documented closest equivalents) are available.
- Map technical series names to plain-language labels in the API so the UI never requires CPI literacy.
- If a feed fails: show vote line if available; mark official lines unavailable.

Exact region list and series IDs are fixed in the implementation plan after source verification.

## 9. Components

| Component | Role |
|-----------|------|
| `ag/` Argent application | Vote slot, deposit, commitment |
| Rust binaries / lib | Tx construction, local verification helpers |
| Web frontend | Wallet, form, chart, consent |
| API + indexer | Verify, encrypt, aggregate, GDPR endpoints |
| Inflation worker | Scheduled fetch + normalize |

Follow Veritas `agents.md`: Argent source of truth is the local Argent checkout; no invented Argent APIs.

## 10. Errors

| Situation | Behaviour |
|-----------|-----------|
| Wallet missing / signature rejected | Clear retry; no partial accept |
| Already voted this month | Block submit; show next open month + deposit status |
| Deposit / covenant tx failed | Do not store off-chain answer |
| Commitment mismatch | Indexer rejects |
| Inflation outage | Vote line OK; official lines flagged down |
| Below k-anonymity threshold | No vote line point for that month/filter |

## 11. Security

- Covenant enforces one vote per wallet per month and deposit rules.
- Indexer never trusts the client alone: requires matching on-chain commitment + wallet proof.
- Secrets only in env / secret store; never committed.
- TLS for API; rate limits on auth and vote.
- Testnet endpoints only in v1 configuration.

## 12. Testing

- Covenant: duplicate month vote fails; deposit release next month succeeds; invalid release fails.
- Commitment encoding: golden vectors shared by client and indexer.
- Indexer: mismatch rejected; erase removes encrypted row; aggregates API never returns wallet or raw answer.
- Chart: filters; threshold behaviour; plain-language labels present.
- Local Argent build/inspect/check loop per project engineering rules before testnet attempts.

## 13. Success criteria

1. A testnet user can connect a Kaspa wallet, consent, vote once per month, and see deposit locked.
2. A second vote the same month fails.
3. Next month, deposit can be reclaimed after a valid prior vote.
4. Chart shows official plain-language lines and vote average for a region with enough responses; employed filter works.
5. Erase removes off-chain personal answer data; on-chain commitment remains; public API still does not expose that wallet’s answer.
6. Full answer bodies are not broadcast as bulky on-chain payloads.

## 14. Open points for implementation plan

- Exact deposit amount (testnet KAS)
- Exact grace period and abandoned-deposit handling
- Wallet connector set (which testnet wallets)
- Final region list and official series mapping
- Encryption scheme for indexer payloads (wallet-keyed or server envelope + wallet auth)
- Argent actor/state shape (to be derived from current Argent `master` / examples, not assumed)
