//! Veritas: verifiable price records anchored on Kaspa L1.
//!
//! The chain proves *when*; the holder proves *what*. A price record is hashed
//! and signed on the attester's device, batched into a Merkle tree, and each
//! batch root is folded into an append-only accumulator that a single Silverscript
//! covenant UTXO carries on Kaspa (`covenants/anchor.sil`).
//!
//! Module map:
//! - [`units`]: printed quantities and exact unit conversion.
//! - [`record`]: the price record, its canonical encoding and signature.
//! - [`merkle`]: per-batch Merkle tree and inclusion proofs.
//! - [`accumulator`]: the hash chain of batch roots held by the covenant.
//! - [`receipt`]: what an attester keeps next to their record.
//! - [`verify`]: the yes/no check anyone can run.
//! - [`pulse`]: legacy cost-of-living vote commitment used by `veritas-api`.

pub mod accumulator;
pub mod hash;
pub mod hex;
pub mod merkle;
pub mod pulse;
pub mod receipt;
pub mod record;
pub mod units;
pub mod verify;

pub use accumulator::{AnchorLog, GENESIS_ACC, fold, next_acc};
pub use merkle::{Batch, MerkleError, Side, Step, root_from_proof};
pub use pulse::{COMMITMENT_DOMAIN, CommitmentError, VoteAnswer, canonical_bytes, commitment_hash};
pub use receipt::Receipt;
pub use record::{PriceRecord, RecordError, SignedRecord, Source};
pub use units::{Dimension, Unit};
pub use verify::{Confirmation, NoReason, Verdict, verify};
