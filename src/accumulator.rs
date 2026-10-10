//! The append-only accumulator carried by the anchor covenant.
//!
//! `acc_0 = 0^32` and `acc_{n+1} = BLAKE2b-256(acc_n || batch_root_n)`.
//! `covenants/anchor.sil` computes exactly this in Kaspa script, so the chain
//! itself refuses any update that is not an append. Anyone holding the public
//! list of batch roots can fold it and compare the result with the live UTXO.

use serde::{Deserialize, Serialize};

use crate::{hash::blake2b256, hex};

pub const GENESIS_ACC: [u8; 32] = [0; 32];

/// One append step. Must match `append` in `covenants/anchor.sil`.
pub fn next_acc(acc: &[u8; 32], batch_root: &[u8; 32]) -> [u8; 32] {
    blake2b256(&[acc, batch_root])
}

/// Folds batch roots in order from the genesis accumulator.
pub fn fold(batch_roots: &[[u8; 32]]) -> [u8; 32] {
    batch_roots
        .iter()
        .fold(GENESIS_ACC, |acc, root| next_acc(&acc, root))
}

/// The public, mirrorable list of every anchored batch root.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnchorLog {
    #[serde(with = "hex::array_list")]
    pub batch_roots: Vec<[u8; 32]>,
}

impl AnchorLog {
    /// Appends a batch root and returns `(batch_index, new_acc)`.
    pub fn append(&mut self, batch_root: [u8; 32]) -> (u64, [u8; 32]) {
        self.batch_roots.push(batch_root);
        (self.batch_roots.len() as u64 - 1, self.acc())
    }

    /// The accumulator the covenant should hold after every listed batch.
    pub fn acc(&self) -> [u8; 32] {
        fold(&self.batch_roots)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_log_is_genesis() {
        assert_eq!(AnchorLog::default().acc(), GENESIS_ACC);
    }

    #[test]
    fn order_matters_and_history_cannot_be_rewritten() {
        let a = [1; 32];
        let b = [2; 32];
        assert_ne!(fold(&[a, b]), fold(&[b, a]));
        assert_ne!(fold(&[a, b]), fold(&[a, [3; 32]]));
        assert_ne!(fold(&[a, b]), fold(&[a]));
    }

    #[test]
    fn append_tracks_index_and_acc() {
        let mut log = AnchorLog::default();
        assert_eq!(log.append([1; 32]), (0, next_acc(&GENESIS_ACC, &[1; 32])));
        let (index, acc) = log.append([2; 32]);
        assert_eq!(index, 1);
        assert_eq!(acc, fold(&[[1; 32], [2; 32]]));
    }
}
