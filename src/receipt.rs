//! The receipt an attester keeps next to their signed record.

use serde::{Deserialize, Serialize};

use crate::{hex, merkle::Step};

pub const RECEIPT_VERSION: u8 = 1;

/// Where a record sits: its path to the batch root and the batch's position in
/// the anchor log. Together with the record and the public log, this is enough
/// to verify against any Kaspa node.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Receipt {
    pub version: u8,
    /// Position of the batch in the anchor log (0-based).
    pub batch_index: u64,
    #[serde(with = "hex::array")]
    pub batch_root: [u8; 32],
    /// Position of the record inside its batch (informational).
    pub leaf_index: u64,
    pub proof: Vec<Step>,
    /// Transaction that anchored the batch, once known (informational; the
    /// proof does not depend on it).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub anchor_txid: Option<String>,
}
