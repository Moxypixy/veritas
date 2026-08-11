# Task 8 Report: Web consent and vote form

## Status

Implemented a Vite, React, strict TypeScript web application under `web/`.

- The first screen presents the consent and trust notice, clearly distinguishing
  the immutable on-chain commitment/deposit from the erasable encrypted server
  record, and describes the Option A indexer trust boundary.
- The form contains the configured regions, necessities percentage, employment
  status, a dynamic duration label, the 0.1 KAS testnet deposit, and two
  explicit confirmations.
- Native form controls, associated labels, semantic fieldsets, visible keyboard
  focus, live status feedback, and reduced-motion styling support WCAG 2.2 AA.
- The wallet control is deliberately a non-signing stub. No wallet, private
  key, transaction, or network integration was added.
- `web/src/commitment.ts` is a domain-only TypeScript encoder. Its Vitest suite
  copies the Rust golden vectors and proves the canonical byte encoding and
  BLAKE2b-256 digests match exactly.

## Verification

Executed in `web/`:

```text
npm test      1 test file, 3 tests passed
npm run lint  passed (tsc --noEmit)
npm run build passed (tsc and Vite production build)
```

## Security and scope review

- This task makes no security-sensitive action possible: it cannot submit,
  sign, connect a wallet, or transmit a full answer.
- The UI does not claim one-vote uniqueness is enforced on-chain; it explicitly
  identifies the indexer as the Option A UTC-month/uniqueness gate.
- Region choices are copied from the existing verified inflation configuration.
- Chart UI and real wallet integration remain out of scope for Tasks 9 and 10.
