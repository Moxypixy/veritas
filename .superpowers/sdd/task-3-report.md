# Task 3 Report: Argent vote + deposit covenant

## Status

DONE

## Implemented

`ag/vote.ag` app `Pulse` with:

- `BallotOffice.open_ballot` — opens per-wallet `VoterBallot`
- `VoterBallot.vote` — one vote per month (`month_yyyymm > last_month`), locks `DEPOSIT_SOMPI` (10_000_000), stores `commitment`
- `VoteDeposit.reclaim` — voter-only, unlock month through grace; emits `WalletPurse` with full value
- `VoteDeposit.abandon` — after grace (vote month + 3), anyone, `emits none`
- `WalletPurse.close` — voter spends reclaimed funds

Month comparisons use entry arg `current_yyyymm` (documented; not chain clock).

## Verification

- `.\argentc.cmd build ag/vote.ag` → success (`wrote build/argent`)
- `inspect` not available in this argentc build

## Commit

- `189e440` Add Argent vote deposit covenant for monthly commitments.

## Critical-review remediation

### Applied

- `VoteDepositState` now records `unlock_yyyymm` and `abandon_yyyymm` when the
  vote is created. `reclaim` and `abandon` validate the supplied month against
  those stored thresholds, rather than accepting an arbitrary later month.
- `abandon` now emits the entire deposit to a fixed `Burn` actor and requires
  `burn.value == self.value`. `Burn.reject_spend` is intentionally
  unsatisfiable (`require(0 == 1)`), which is the compiler-supported
  unspendable output pattern; a no-entry actor was rejected by SilverScript.
- Comments document the absent on-chain UTC/DAA source. The later indexer/API
  must reject requests whose supplied month does not equal the UTC month it
  observes.

### Limitation requiring a design change

This Argent checkout has no map/set state, keyed actor lookup, or
negative-UTXO query. Therefore a global `wallet -> ballot` uniqueness
invariant cannot be expressed or proved in `ag/vote.ag`: a caller can still
open a second independent `VoterBallot` from `BallotOffice`. The existing
single-ballot lineage prevents a duplicate vote only within that lineage.
Do not deploy this covenant as one-vote-per-wallet until a verified
registry/identity primitive or trusted issuance boundary is added.

### Verification

- `.\argentc.cmd build ag\vote.ag` → success: `wrote build/argent`
- First burn-actor attempt with no entrypoint was rejected by the current
  compiler: `unsupported feature: contract has no entrypoint functions`.
