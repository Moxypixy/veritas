# Veritas

Verifiable price records on Kaspa L1. Veritas answers one question with a yes
or a no: **was this the price, then?**

The chain proves *when*; the holder proves *what*. A price is hashed and signed
on the attester's device, batched into a Merkle tree, and every batch root is
folded into an append-only accumulator carried by a single
[Silverscript](https://github.com/kaspanet/silverscript) covenant UTXO. The
UTXO set is never pruned, so any ordinary Kaspa node can check the anchor for
as long as the covenant lives. Records and receipts stay with the people who
made them.

Two verified prices for the same item, months apart, give a personal inflation
figure that can be set beside the official CPI. No token, no fee beyond Kaspa
transaction costs, open source (ISC).

Author: Moxy ([github.com/Moxypixy](https://github.com/Moxypixy)).

## How it works

1. **Attest.** A price record (item, amount in minor units, quantity as
   printed, unit, place, date, source, evidence hash, random salt) is encoded
   canonically, hashed with BLAKE2b-256 and signed with a BIP-340 Schnorr key.
2. **Batch.** Leaves go into a domain-separated Merkle tree. Each attester
   gets a receipt: their Merkle path, the batch root and the batch's position.
3. **Anchor.** The batch root is appended to the accumulator:
   `acc' = blake2b(acc || batch_root)`. The covenant in
   [`covenants/anchor.sil`](covenants/anchor.sil) computes this itself and
   refuses any successor that is not an append, signed by the batcher key.
4. **Verify.** With the record, its receipt, the public list of batch roots
   and the accumulator read from the live UTXO, anyone can check: signature,
   Merkle path, position in the log, and that the log folds to the live root.
   All pass: YES. Anything else: NO, with the reason.

The public batch-root list is 32 bytes per batch. Anyone can mirror it and no
one needs to trust a mirror, because it must fold to the root the covenant
holds.

### What a YES means

| `source` | A YES proves |
| --- | --- |
| `live` | The price was attested at or near the time it was observed. |
| `archive` | The price appeared on a page captured by an independent archive at that archive's timestamp. |
| `attested_late` | The record existed by its anchoring time. The printed date is not vouched for. |

Veritas proves who claimed what, and when. It does not prove a claim is true.

### Units

Quantities are stored exactly as printed (`qty_milli` thousandths of `unit`)
and converted to a price per litre, kilogram or item with exact integer
factors. Ambiguous units are rejected: `us_pt` and `imp_pt` exist, `pt` does
not, because a UK pint is about 20% larger than a US pint.

| Code | Size |
| --- | --- |
| `l`, `ml` | 1 L, 1 mL |
| `us_gal`, `us_pt` | 3.785411784 L, 0.473176473 L |
| `imp_gal`, `imp_pt` | 4.54609 L, 0.56826125 L |
| `kg`, `g`, `lb`, `oz` | 1 kg, 1 g, 0.45359237 kg, 0.028349523125 kg |
| `each` | 1 item |

Amounts are integers in minor units (øre, cents) so every device hashes the
same bytes. Currency is never converted inside a record.

## Repository layout

| Path | What it is |
| --- | --- |
| [`covenants/anchor.sil`](covenants/anchor.sil) | The anchor covenant (Silverscript). |
| [`src/`](src) | Crate `veritas`: records, units, Merkle batches, accumulator, receipts, verification. Pure Rust, no Kaspa dependency. |
| [`src/bin/veritas.rs`](src/bin/veritas.rs) | Demo CLI. |
| [`crates/veritas-anchor`](crates/veritas-anchor) | Compiles the covenant per state and builds `append` sigscripts. Its tests execute the covenant in rusty-kaspa's script engine. |
| [`web/`](web) | React client. `/` is the browser verifier; it mirrors the Rust verifier byte for byte. |
| [`examples/demo`](examples/demo) | Committed demo records. Rust and web tests both verify them. |
| [`crates/veritas-api`](crates/veritas-api), [`crates/veritas-inflation`](crates/veritas-inflation) | Legacy Cost-of-Living Pulse API and official-statistics worker, kept working; planned to move to The Gap. |
| [`docs/specs`](docs/specs) | Design spec for v0. |

## Quick start

Requires Rust (pinned in `rust-toolchain.toml`) and, for the web client,
Node.js 22+.

```bash
cargo test --workspace          # core, covenant-in-script-engine and legacy tests
cargo build --bin veritas
```

Walk through the demo with the CLI:

```bash
B=target/debug/veritas
mkdir -p /tmp/veritas && cd /tmp/veritas

$B keygen attester.key
$B attest attester.key $OLDPWD/examples/milk-2026-03.json march.signed.json
$B batch anchor-log.json receipts march.signed.json
$B attest attester.key $OLDPWD/examples/milk-2026-10.json october.signed.json
$B batch anchor-log.json receipts october.signed.json      # prints "anchor to <acc>"

$B verify march.signed.json receipts/march.receipt.json anchor-log.json <acc>   # YES
$B unit-price march.signed.json
```

Change one digit of `amount_minor` in `march.signed.json` and verify again: NO.

Browser verifier:

```bash
cd web && npm install && npm run dev
```

Open the page, choose **Load the milk demo**, then **Verify**. Edit a digit
and verify again.

## Kaspa and Silverscript versions

- Covenant language: Silverscript `v1.0.0` (kaspanet/silverscript).
- rusty-kaspa: revision `a41a333` (Toccata, transaction v1, covenant IDs), the
  revision Silverscript 1.0.0 is built and tested against. The released
  `v2.0.1` tag predates a crate Silverscript needs.
- Toccata activated on Kaspa mainnet on 30 June 2026. Silverscript recommends
  testnet-10 until its tooling matures; Veritas v0 targets testnet-10.

## Status and limits (v0)

Done:

- Price record format, units, Merkle batches, accumulator, receipts, verifier.
- Anchor covenant in Silverscript, tested in Kaspa's script engine: honest
  appends pass; rewriting history, a foreign key, handing over the batcher
  key, forking into two anchors and deleting the anchor are rejected.
- CLI demo and a browser verifier that agrees with Rust on committed fixtures.

Not yet:

- Submitting the anchoring transaction to a node (needs a funded testnet
  wallet, compute budget and fee handling), and reading the live accumulator
  from a node instead of pasting it.
- One batcher key per anchor; no key rotation, no open appends.
- Attester reputation and Sybil resistance (planned with LAP).
- Item catalogue (fat %, organic, pack-size bands) beyond free-text item ids.
- Passkey (P-256) attesters; v0 uses Schnorr keys like Kaspa wallets.

## Relation to The Gap and LAP

Veritas is the evidence layer. The Gap consumes verified prices for its
cost-of-living burden index. LAP adds verified labour and income, and later
gives attesters a reputation that weights their prices. One anchor, three
protocols.
