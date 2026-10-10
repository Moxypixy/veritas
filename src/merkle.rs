//! Per-batch Merkle tree over record leaves.
//!
//! Hashes use the KCC-1 Hash (BLAKE3-256). Node hashes are domain-tagged so a leaf can never be passed off as an inner
//! node: `leaf_node = H(0x00 || leaf)`, `parent = H(0x01 || left || right)`.
//! An odd node at the end of a level is promoted unchanged (never duplicated),
//! so no two different leaf lists share a root.

use serde::{Deserialize, Serialize};
use std::fmt;

use crate::{hash::kcc_hash, hex};

const LEAF_TAG: [u8; 1] = [0x00];
const NODE_TAG: [u8; 1] = [0x01];

/// Which side the sibling sits on when folding a proof upwards.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Side {
    Left,
    Right,
}

/// One proof step: a sibling hash and its side.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Step {
    pub side: Side,
    #[serde(with = "hex::array")]
    pub hash: [u8; 32],
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MerkleError {
    EmptyBatch,
    IndexOutOfRange,
}

impl fmt::Display for MerkleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            MerkleError::EmptyBatch => "a batch needs at least one record",
            MerkleError::IndexOutOfRange => "leaf index is outside the batch",
        })
    }
}

impl std::error::Error for MerkleError {}

pub fn leaf_node(leaf: &[u8; 32]) -> [u8; 32] {
    kcc_hash(&[&LEAF_TAG, leaf])
}

pub fn parent(left: &[u8; 32], right: &[u8; 32]) -> [u8; 32] {
    kcc_hash(&[&NODE_TAG, left, right])
}

/// A built batch tree, kept whole so proofs can be cut for every record.
#[derive(Clone, Debug)]
pub struct Batch {
    levels: Vec<Vec<[u8; 32]>>,
}

impl Batch {
    pub fn new(leaves: &[[u8; 32]]) -> Result<Self, MerkleError> {
        if leaves.is_empty() {
            return Err(MerkleError::EmptyBatch);
        }
        let mut levels = vec![leaves.iter().map(leaf_node).collect::<Vec<_>>()];
        while levels.last().map_or(0, Vec::len) > 1 {
            let level = levels.last().expect("non-empty");
            let next = level
                .chunks(2)
                .map(|pair| match pair {
                    [left, right] => parent(left, right),
                    [single] => *single,
                    _ => unreachable!("chunks(2) yields one or two items"),
                })
                .collect();
            levels.push(next);
        }
        Ok(Self { levels })
    }

    pub fn len(&self) -> usize {
        self.levels[0].len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn root(&self) -> [u8; 32] {
        self.levels.last().expect("at least one level")[0]
    }

    /// The inclusion proof for the leaf at `index`, bottom-up.
    pub fn proof(&self, index: usize) -> Result<Vec<Step>, MerkleError> {
        if index >= self.len() {
            return Err(MerkleError::IndexOutOfRange);
        }
        let mut steps = Vec::new();
        let mut position = index;
        for level in &self.levels[..self.levels.len() - 1] {
            let sibling = position ^ 1;
            if let Some(hash) = level.get(sibling) {
                let side = if sibling < position {
                    Side::Left
                } else {
                    Side::Right
                };
                steps.push(Step { side, hash: *hash });
            }
            position /= 2;
        }
        Ok(steps)
    }
}

/// Folds a record leaf and its proof up to the batch root.
pub fn root_from_proof(leaf: &[u8; 32], proof: &[Step]) -> [u8; 32] {
    proof
        .iter()
        .fold(leaf_node(leaf), |node, step| match step.side {
            Side::Left => parent(&step.hash, &node),
            Side::Right => parent(&node, &step.hash),
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn leaves(count: u8) -> Vec<[u8; 32]> {
        (0..count).map(|i| [i; 32]).collect()
    }

    #[test]
    fn every_proof_folds_to_the_root_for_many_sizes() {
        for count in 1..=17 {
            let leaves = leaves(count);
            let batch = Batch::new(&leaves).unwrap();
            for (index, leaf) in leaves.iter().enumerate() {
                let proof = batch.proof(index).unwrap();
                assert_eq!(
                    root_from_proof(leaf, &proof),
                    batch.root(),
                    "count {count}, index {index}"
                );
            }
        }
    }

    #[test]
    fn single_leaf_root_is_the_tagged_leaf() {
        let batch = Batch::new(&[[9; 32]]).unwrap();
        assert_eq!(batch.root(), leaf_node(&[9; 32]));
        assert!(batch.proof(0).unwrap().is_empty());
    }

    #[test]
    fn wrong_leaf_or_side_does_not_reach_the_root() {
        let leaves = leaves(5);
        let batch = Batch::new(&leaves).unwrap();
        let proof = batch.proof(2).unwrap();
        assert_ne!(root_from_proof(&[42; 32], &proof), batch.root());

        let mut flipped = proof.clone();
        flipped[0].side = match flipped[0].side {
            Side::Left => Side::Right,
            Side::Right => Side::Left,
        };
        assert_ne!(root_from_proof(&leaves[2], &flipped), batch.root());
    }

    #[test]
    fn odd_node_is_promoted_not_duplicated() {
        let two = Batch::new(&leaves(2)).unwrap().root();
        let mut three_leaves = leaves(2);
        three_leaves.push(three_leaves[1]);
        assert_ne!(Batch::new(&three_leaves).unwrap().root(), two);
    }

    #[test]
    fn rejects_empty_batches_and_bad_indexes() {
        assert_eq!(Batch::new(&[]).unwrap_err(), MerkleError::EmptyBatch);
        assert_eq!(
            Batch::new(&leaves(3)).unwrap().proof(3).unwrap_err(),
            MerkleError::IndexOutOfRange
        );
    }
}
