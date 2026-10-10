//! BLAKE2b-256, the hash Kaspa script exposes as `blake2b` / `OpBlake2b`.

use blake2b_simd::Params;

/// Hashes the concatenation of `parts` with unkeyed BLAKE2b-256.
pub fn blake2b256(parts: &[&[u8]]) -> [u8; 32] {
    let mut state = Params::new().hash_length(32).to_state();
    for part in parts {
        state.update(part);
    }
    let mut out = [0; 32];
    out.copy_from_slice(state.finalize().as_bytes());
    out
}
