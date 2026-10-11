# Veritas

Verifiable price records on Kaspa L1. Veritas answers one question with a yes
or a no: **was this the price, then?**

The chain proves *when*; the holder proves *what*. A price is hashed and signed
on the attester's device (in the browser with the
[Kaspire](https://kaspire.kaslab.space/) wallet, or with the CLI), batched into a Merkle tree, and every batch root is
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
   canonically, hashed with the KCC-1 Hash (BLAKE3-256) and signed by the
   attester with KIP-5 personal-message signing. The wallet shows a readable
   summary that ends with the record's hash, so the attester sees what they
   sign.
2. **Batch.** Leaves go into a domain-separated Merkle tree. Each attester
   gets a receipt: their Merkle path, the batch root and the batch's position.
3. **Anchor.** The batch root is appended to the accumulator:
   `acc' = Hash(acc || batch_root)` (BLAKE3). The covenant in
   [`covenants/anchor.sil`](covenants/anchor.sil) computes this itself and
   refuses any successor that is not an append approved by the batcher
   authority.
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

### Privacy

- Evidence photos are cleaned on the device before hashing: the page
  re-encodes the image to pixels only and refuses any output that still
  contains EXIF, XMP, IPTC or comment data. Only the cleaned copy's hash
  enters the record; the attester keeps that cleaned copy. Verified in
  Chromium 141 with a GPS-tagged JPEG and PNG; other browsers rely on the
  built-in check, which rejects any output that still has metadata.
- Place is an area (municipality or region) by default. The shop name is
  stored only if the attester opts in, because exact shops across many
  records can point to where someone lives.

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

## Kaspa conventions

Veritas issues no token. It follows the Kaspa Calls for Conventions
([kaspanet/kccs](https://github.com/kaspanet/kccs)) so wallets and indexers can
read the anchor like any other covenant:

| Convention | How Veritas follows it |
| --- | --- |
| KCC-1 Covenant Concepts, Byte Layouts and ABI | The anchor is compiled by Silverscript 1.0, which emits KCC-1 state encoding, dispatch tags (`append` = `c6b9047f`) and the P2SH envelope. The singleton declaration validates the continuation's program, state and covenant binding. Veritas commitments use the KCC-1 Hash Function (BLAKE3-256). |
| KCC-1 Program ABI | Published in [`covenants/anchor.abi.json`](covenants/anchor.abi.json). The template hash, the same for every anchor state, is in [`covenants/anchor.template-hash.txt`](covenants/anchor.template-hash.txt); a test fails if it drifts from the source. |
| KCC-2 Authority Schemes | The batcher is an authority with a canonical scheme byte: `0x00` `p2pk-schnorr/v1` (a key) or `0x04` `covenant-id/v1` (a governance or multisig covenant must take part). Other bytes are rejected on chain. |
| KCC-20 Fungible Token | Not a token, so not a KCC-20 instance. The anchor reuses KCC-20's owner-authentication witness layout: 65-byte signature for `p2pk-schnorr/v1`, empty for `covenant-id/v1`. |
| KIP-5 Message Signing | Attesters sign records, and Pulse users sign logins, with KIP-5; checked against the official KIP-5 test vectors in Rust and TypeScript. |
| KIP-20 Covenant IDs | The anchor is one covenant lineage; wallets identify it by Covenant ID plus the template hash above. |

## Wallet: Kaspire

The web client uses the [Kaspire](https://kaspire.kaslab.space/) browser
extension (KaspaHUB21) through its `window.kaspire` provider:

- **Attest a price** (`/attest`): `requestAccounts` and `getPublicKey` make
  your Kaspire key the attester; `signMessage` signs the record with KIP-5.
  The page verifies the signature locally before offering the file. The
  receipt photo is hashed in the browser and never uploaded.
- **Verify a price** (`/`): no wallet needed.
- **Pulse vote** (`/pulse`): logs in with a KIP-5 signature over a server
  challenge. The API accepts it only for a `kaspatest:` address that encodes
  the signing key.

Kaspire's extension documents Testnet-10 support but its store build may
report `mainnet`. Attestation works on any network, since it signs a message
rather than a transaction; Pulse login requires Testnet-10.

## Repository layout

| Path | What it is |
| --- | --- |
| [`covenants/anchor.sil`](covenants/anchor.sil) | The anchor covenant (Silverscript). |
| [`src/`](src) | Crate `veritas`: records, units, Merkle batches, accumulator, receipts, verification. Pure Rust, no Kaspa dependency. |
| [`src/bin/veritas.rs`](src/bin/veritas.rs) | Demo CLI. |
| [`crates/veritas-anchor`](crates/veritas-anchor) | Compiles the covenant per state, builds `append` sigscripts and regenerates the published ABI (`cargo run -p veritas-anchor --bin anchor-abi`). Its tests execute the covenant in rusty-kaspa's script engine. |
| [`web/`](web) | React client: verifier at `/`, Kaspire attestation at `/attest`. `src/veritas/verify.ts` mirrors the Rust verifier byte for byte. |
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
  appends pass for both a key and a covenant-ID batcher; rewriting history, a
  foreign key or lineage, malformed witnesses, unsupported KCC-2 schemes,
  handing over the batcher, forking into two anchors and deleting the anchor
  are rejected.
- Kaspire attestation and Pulse login, with a KIP-5 login verifier in the API.
- CLI demo and a browser verifier that agrees with Rust on committed fixtures.

Not yet:

- Submitting the anchoring transaction to a node (needs a funded testnet
  wallet, compute budget and fee handling), and reading the live accumulator
  from a node instead of pasting it.
- Creating the anchor's genesis (KIP-20) and building the `append`
  transaction for Kaspire's `signPskt`.
- One batcher authority per anchor; no rotation, no open appends.
- Attester reputation and Sybil resistance (planned with LAP).
- Item catalogue (fat %, organic, pack-size bands) beyond free-text item ids.
- Passkey (P-256) attesters; v0 uses Schnorr keys like Kaspa wallets.

## Relation to The Gap and LAP

Veritas is the evidence layer. The Gap consumes verified prices for its
cost-of-living burden index. LAP adds verified labour and income, and later
gives attesters a reputation that weights their prices. One anchor, three
protocols.
