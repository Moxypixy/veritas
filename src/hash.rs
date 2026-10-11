//! The KCC-1 Hash Function: unkeyed BLAKE3 with a 32-byte output.
//!
//! Every Veritas-defined commitment (record leaves, Merkle nodes, the anchor
//! accumulator) uses it, matching `blake3(...)` in Silverscript and the hash
//! KCC-1 and KCC-2 specify for conventions. KIP-5 message signing keeps its
//! own keyed BLAKE2b (see [`crate::kip5`]).

/// `Hash(parts[0] || parts[1] || ...)` per KCC-1 Section 3.2.1.
pub fn kcc_hash(parts: &[&[u8]]) -> [u8; 32] {
    let mut hasher = blake3::Hasher::new();
    for part in parts {
        hasher.update(part);
    }
    *hasher.finalize().as_bytes()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_the_blake3_reference_for_empty_input() {
        // BLAKE3 test vector: hash of the empty string.
        assert_eq!(
            crate::hex::encode(&kcc_hash(&[])),
            "af1349b9f5f9a1a6a0404dea36dcc9499bcb25c9adc112b7cc9a93cae41f3262"
        );
    }

    #[test]
    fn concatenates_parts() {
        assert_eq!(kcc_hash(&[b"ab", b"c"]), kcc_hash(&[b"abc"]));
    }
}
