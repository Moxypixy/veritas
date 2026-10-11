//! The yes/no check: was this the price, then?

use std::fmt;

use crate::{
    accumulator::fold, merkle::root_from_proof, receipt::Receipt, record::SignedRecord,
    record::Source,
};

/// What a YES vouches for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Confirmation {
    pub batch_index: u64,
    pub attester: [u8; 32],
    pub observed_on: String,
    pub source: Source,
    /// Price per litre, kilogram or item in thousandths of the minor unit.
    pub price_per_base_milli: Option<u128>,
}

impl Confirmation {
    /// Plain-language statement of how far the date can be trusted.
    pub fn time_claim(&self) -> &'static str {
        match self.source {
            Source::Live => "price attested at or near the observation date",
            Source::Archive => {
                "price shown on an independently archived page at the archive's timestamp"
            }
            Source::AttestedLate => {
                "record existed by its anchoring time; the printed date is not vouched for"
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NoReason {
    InvalidRecord(String),
    BadSignature,
    UnsupportedReceipt(u8),
    ProofDoesNotReachBatchRoot,
    BatchNotInLog,
    LogDoesNotMatchLiveAnchor,
}

impl fmt::Display for NoReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NoReason::InvalidRecord(why) => write!(f, "record is malformed: {why}"),
            NoReason::BadSignature => {
                f.write_str("signature does not match the record and attester")
            }
            NoReason::UnsupportedReceipt(v) => write!(f, "receipt version {v} is not supported"),
            NoReason::ProofDoesNotReachBatchRoot => {
                f.write_str("record is not in the batch its receipt names")
            }
            NoReason::BatchNotInLog => {
                f.write_str("batch root is not in the anchor log at that position")
            }
            NoReason::LogDoesNotMatchLiveAnchor => {
                f.write_str("anchor log does not hash to the live covenant root")
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Verdict {
    Yes(Confirmation),
    No(NoReason),
}

/// Verifies a record against its receipt, the public log of batch roots and
/// the accumulator read from the live anchor UTXO.
pub fn verify(
    signed: &SignedRecord,
    receipt: &Receipt,
    batch_roots: &[[u8; 32]],
    live_acc: &[u8; 32],
) -> Verdict {
    let leaf = match signed.verified_leaf() {
        Ok(Some(leaf)) => leaf,
        Ok(None) => return Verdict::No(NoReason::BadSignature),
        Err(error) => return Verdict::No(NoReason::InvalidRecord(error.to_string())),
    };
    if receipt.version != crate::receipt::RECEIPT_VERSION {
        return Verdict::No(NoReason::UnsupportedReceipt(receipt.version));
    }
    if root_from_proof(&leaf, &receipt.proof) != receipt.batch_root {
        return Verdict::No(NoReason::ProofDoesNotReachBatchRoot);
    }
    let listed = usize::try_from(receipt.batch_index)
        .ok()
        .and_then(|index| batch_roots.get(index));
    if listed != Some(&receipt.batch_root) {
        return Verdict::No(NoReason::BatchNotInLog);
    }
    if &fold(batch_roots) != live_acc {
        return Verdict::No(NoReason::LogDoesNotMatchLiveAnchor);
    }
    let record = &signed.record;
    Verdict::Yes(Confirmation {
        batch_index: receipt.batch_index,
        attester: record.attester,
        observed_on: record.observed_on.clone(),
        source: record.source,
        price_per_base_milli: record.price_per_base_milli(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        accumulator::AnchorLog,
        merkle::Batch,
        receipt::RECEIPT_VERSION,
        record::tests::{milk, test_keypair},
    };

    struct World {
        old: SignedRecord,
        old_receipt: Receipt,
        new: SignedRecord,
        new_receipt: Receipt,
        log: AnchorLog,
    }

    fn receipt_for(batch: &Batch, batch_index: u64, leaf_index: usize) -> Receipt {
        Receipt {
            version: RECEIPT_VERSION,
            batch_index,
            batch_root: batch.root(),
            leaf_index: leaf_index as u64,
            proof: batch.proof(leaf_index).unwrap(),
            anchor_txid: None,
        }
    }

    /// The demo story: milk attested in March, again in October.
    fn world() -> World {
        let alice = test_keypair(1);
        let bob = test_keypair(2);
        let mut log = AnchorLog::default();

        let old = milk(&alice, 1_195, "2026-03-02").sign(&alice).unwrap();
        let other = milk(&bob, 1_250, "2026-03-02").sign(&bob).unwrap();
        let march = Batch::new(&[
            other.verified_leaf().unwrap().unwrap(),
            old.verified_leaf().unwrap().unwrap(),
        ])
        .unwrap();
        let (march_index, _) = log.append(march.root());
        let old_receipt = receipt_for(&march, march_index, 1);

        let new = milk(&alice, 1_295, "2026-10-10").sign(&alice).unwrap();
        let october = Batch::new(&[new.verified_leaf().unwrap().unwrap()]).unwrap();
        let (october_index, _) = log.append(october.root());
        let new_receipt = receipt_for(&october, october_index, 0);

        World {
            old,
            old_receipt,
            new,
            new_receipt,
            log,
        }
    }

    #[test]
    fn both_prices_verify_yes() {
        let w = world();
        let acc = w.log.acc();
        let Verdict::Yes(old) = verify(&w.old, &w.old_receipt, &w.log.batch_roots, &acc) else {
            panic!("old price should verify");
        };
        let Verdict::Yes(new) = verify(&w.new, &w.new_receipt, &w.log.batch_roots, &acc) else {
            panic!("new price should verify");
        };
        assert_eq!(old.batch_index, 0);
        assert_eq!(new.batch_index, 1);
        assert_eq!(old.price_per_base_milli, Some(1_195_000));
        assert_eq!(new.price_per_base_milli, Some(1_295_000));
    }

    #[test]
    fn changing_one_digit_says_no() {
        let w = world();
        let mut edited = w.old.clone();
        edited.record.amount_minor = 995;
        assert_eq!(
            verify(&edited, &w.old_receipt, &w.log.batch_roots, &w.log.acc()),
            Verdict::No(NoReason::BadSignature)
        );
    }

    #[test]
    fn re_signed_edit_is_not_in_the_batch() {
        let w = world();
        let alice = test_keypair(1);
        let mut record = w.old.record.clone();
        record.amount_minor = 995;
        let forged = record.sign(&alice).unwrap();
        assert_eq!(
            verify(&forged, &w.old_receipt, &w.log.batch_roots, &w.log.acc()),
            Verdict::No(NoReason::ProofDoesNotReachBatchRoot)
        );
    }

    #[test]
    fn a_rewritten_log_fails_against_the_live_anchor() {
        let w = world();
        let live = w.log.acc();
        let mut rewritten = w.log.batch_roots.clone();
        rewritten[1] = [0xee; 32];
        assert_eq!(
            verify(&w.old, &w.old_receipt, &rewritten, &live),
            Verdict::No(NoReason::LogDoesNotMatchLiveAnchor)
        );
    }

    #[test]
    fn receipt_pointing_at_the_wrong_batch_says_no() {
        let w = world();
        let mut receipt = w.old_receipt.clone();
        receipt.batch_index = 1;
        assert_eq!(
            verify(&w.old, &receipt, &w.log.batch_roots, &w.log.acc()),
            Verdict::No(NoReason::BatchNotInLog)
        );
    }

    #[test]
    fn json_round_trip_still_verifies() {
        let w = world();
        let signed: SignedRecord =
            serde_json::from_str(&serde_json::to_string(&w.new).unwrap()).unwrap();
        let receipt: Receipt =
            serde_json::from_str(&serde_json::to_string(&w.new_receipt).unwrap()).unwrap();
        let log: AnchorLog = serde_json::from_str(&serde_json::to_string(&w.log).unwrap()).unwrap();
        assert!(matches!(
            verify(&signed, &receipt, &log.batch_roots, &w.log.acc()),
            Verdict::Yes(_)
        ));
    }
}
