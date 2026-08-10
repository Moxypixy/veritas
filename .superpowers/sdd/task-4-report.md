# Task 4 Report: Local-runtime vote and reclaim binaries

## Delivered

- Added `LocalVoteFixture`, which compiles and loads the current `ag/vote.ag`
  artifact, opens a deterministic fixture wallet's ballot, and builds
  local-runtime transactions for `vote`, `reclaim`, and `abandon`.
- Added `vote_local` and `reclaim_local` binaries. They construct and validate
  local Argent-runtime transactions only; they do not connect to a Kaspa
  network, wallet, signer, or RPC service.
- Reused the Task 2 `commitment_hash` implementation to produce the fixture
  vote commitment.
- Added runtime rule tests covering:
  - first vote success;
  - rejection of a second same-month vote in the same ballot lineage;
  - reclaim rejection during the vote month;
  - reclaim success in the next-month unlock window;
  - abandon rejection before the recorded threshold and success at the
    threshold.

## Verification

The rule test was first run before implementation and failed because
`LocalVoteFixture` did not exist. After implementation, the following fresh
verification command completed with exit code 0:

```powershell
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
cargo run --quiet --bin vote_local
cargo run --quiet --bin reclaim_local
.\check-win.ps1
```

`cargo test --all-features` ran 8 tests: 3 commitment tests and 5 new
local-runtime rule tests, all passing. The binaries each built and executed
their expected transaction.

## Review notes and remaining limitations

- The Rust fixture only orchestrates transactions; the Argent covenant remains
  the authority that accepts or rejects vote, reclaim, and abandon paths.
- Artifact builds are protected by a process-local mutex so Rust's parallel
  test runner cannot concurrently overwrite `build/vote`.
- This is local-runtime coverage only. It verifies no network connectivity,
  wallet custody, signing handoff, broadcast, or confirmation.
- The approved Option A model remains limited to one vote per ballot lineage.
  A trusted indexer/API must enforce one ballot per wallet and verify the
  supplied UTC month, as documented in `ag/vote.ag`.
