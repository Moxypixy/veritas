//! The Veritas anchor covenant, compiled with Silverscript.
//!
//! One covenant UTXO carries `(batcher, batcher_scheme, acc, count)`. Each
//! `append` spends it and must create exactly one successor whose `acc` is
//! `Hash(acc || batch_root)` (KCC-1 Hash, BLAKE3; see
//! [`veritas::accumulator::next_acc`]). The batcher is a KCC-2 authority.
//!
//! Silverscript bakes the state into the P2SH redeem script, so every state has
//! its own program; [`compile`] produces it. The KCC-1 template hash is the
//! same for every state ([`template_hash`]); wallets and indexers use it with
//! the Covenant ID and the published Program ABI (`covenants/anchor.abi.json`)
//! to recognise and decode the anchor.

use kaspa_consensus_core::tx::ScriptPublicKey;
use kaspa_txscript::{EngineFlags, pay_to_script_hash_script, script_builder::ScriptBuilder};
use silverscript_abi::{ArtifactValue, SilAbiArtifact, encode_contract_covenant_decl_sig_script};
use silverscript_lang::compiler::{
    CompileOptions, CompilerError, compile_to_sil_abi_artifact_with_options,
};

pub use silverscript_abi;

/// Source of the covenant, shared with the repository's `covenants/` folder.
pub const ANCHOR_SOURCE: &str = include_str!("../../../covenants/anchor.sil");
pub const CONTRACT_NAME: &str = "VeritasAnchor";
/// Policy function name in `anchor.sil` (the public entry is `append`).
pub const APPEND_DECLARATION: &str = "append_batch";

pub type AnchorResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

/// KCC-2 authority schemes the anchor accepts for its batcher.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AuthorityScheme {
    /// `p2pk-schnorr/v1` (0x00): `batcher` is an x-only Schnorr key; the
    /// witness is a 65-byte transaction signature.
    P2pkSchnorr,
    /// `covenant-id/v1` (0x04): `batcher` is a KIP-20 Covenant ID; an input of
    /// that lineage must be spent in the same transaction; the witness is empty.
    CovenantId,
}

impl AuthorityScheme {
    /// Canonical KCC-2 scheme byte.
    pub fn byte(self) -> u8 {
        match self {
            AuthorityScheme::P2pkSchnorr => 0x00,
            AuthorityScheme::CovenantId => 0x04,
        }
    }

    pub fn from_byte(byte: u8) -> Option<Self> {
        match byte {
            0x00 => Some(AuthorityScheme::P2pkSchnorr),
            0x04 => Some(AuthorityScheme::CovenantId),
            _ => None,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            AuthorityScheme::P2pkSchnorr => "p2pk-schnorr/v1",
            AuthorityScheme::CovenantId => "covenant-id/v1",
        }
    }
}

/// The covenant's state fields, in KCC-1 declaration order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AnchorState {
    /// KCC-2 authority value allowed to append.
    pub batcher: [u8; 32],
    pub batcher_scheme: AuthorityScheme,
    /// Accumulator over every anchored batch root.
    pub acc: [u8; 32],
    /// Number of batches anchored so far.
    pub count: i64,
}

impl AnchorState {
    pub fn genesis(batcher: [u8; 32], batcher_scheme: AuthorityScheme) -> Self {
        Self {
            batcher,
            batcher_scheme,
            acc: veritas::GENESIS_ACC,
            count: 0,
        }
    }

    /// The only successor `append(batch_root, witness)` accepts.
    pub fn after(&self, batch_root: &[u8; 32]) -> Self {
        Self {
            acc: veritas::next_acc(&self.acc, batch_root),
            count: self.count + 1,
            ..*self
        }
    }
}

/// Compiles the covenant for one concrete state.
pub fn compile(state: &AnchorState) -> Result<SilAbiArtifact, Box<CompilerError>> {
    compile_raw(
        state.batcher,
        state.batcher_scheme.byte(),
        state.acc,
        state.count,
    )
}

/// Compiles with a raw scheme byte (used to show unsupported schemes fail).
pub fn compile_raw(
    batcher: [u8; 32],
    batcher_scheme: u8,
    acc: [u8; 32],
    count: i64,
) -> Result<SilAbiArtifact, Box<CompilerError>> {
    let args: [ArtifactValue; 4] = [
        batcher.to_vec().into(),
        batcher_scheme.into(),
        acc.to_vec().into(),
        count.into(),
    ];
    compile_to_sil_abi_artifact_with_options(ANCHOR_SOURCE, &args, CompileOptions::default())
        .map_err(Box::new)
}

fn contract(artifact: &SilAbiArtifact) -> AnchorResult<&silverscript_abi::SilContractArtifact> {
    Ok(artifact
        .contracts
        .get(CONTRACT_NAME)
        .ok_or("artifact has no VeritasAnchor contract")?)
}

/// Redeem script bytes of a compiled state.
pub fn redeem_script(artifact: &SilAbiArtifact) -> AnchorResult<&[u8]> {
    Ok(&contract(artifact)?.compiled.bytecode)
}

/// KCC-1 template hash: identical for every anchor state.
pub fn template_hash(artifact: &SilAbiArtifact) -> AnchorResult<[u8; 32]> {
    Ok(contract(artifact)?.compiled.template_hash)
}

/// P2SH script public key that locks a UTXO to this state.
pub fn script_public_key(artifact: &SilAbiArtifact) -> AnchorResult<ScriptPublicKey> {
    Ok(pay_to_script_hash_script(redeem_script(artifact)?))
}

/// Signature script for `append(batch_root, witness)` spending `current`.
///
/// `witness` follows KCC-20's owner-authentication layout: a 65-byte
/// transaction signature (64 bytes + sighash type) for `p2pk-schnorr/v1`,
/// empty for `covenant-id/v1`.
pub fn append_sig_script(
    current: &SilAbiArtifact,
    batch_root: &[u8; 32],
    witness: &[u8],
) -> AnchorResult<Vec<u8>> {
    let args: [ArtifactValue; 2] = [batch_root.to_vec().into(), witness.to_vec().into()];
    let mut script = encode_contract_covenant_decl_sig_script(
        current,
        CONTRACT_NAME,
        APPEND_DECLARATION,
        false,
        &args,
    )?;
    let push = ScriptBuilder::with_flags(EngineFlags {
        covenants_enabled: true,
        ..Default::default()
    })
    .add_data(redeem_script(current)?)?
    .drain();
    script.extend_from_slice(&push);
    Ok(script)
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEMPLATE_HASH_FILE: &str = include_str!("../../../covenants/anchor.template-hash.txt");

    fn genesis() -> AnchorState {
        AnchorState::genesis([0xaa; 32], AuthorityScheme::P2pkSchnorr)
    }

    #[test]
    fn compiles_and_exposes_append() {
        let artifact = compile(&genesis()).unwrap();
        let contract = &artifact.contracts[CONTRACT_NAME];
        assert!(
            contract.entries.contains_key("append"),
            "entries: {:?}",
            contract.entries.keys()
        );
    }

    #[test]
    fn each_state_has_its_own_program_but_one_template() {
        let first = compile(&genesis()).unwrap();
        let next = compile(&genesis().after(&[1; 32])).unwrap();
        let other_batcher = compile(&AnchorState::genesis(
            [0xbb; 32],
            AuthorityScheme::CovenantId,
        ))
        .unwrap();
        assert_ne!(
            script_public_key(&first).unwrap(),
            script_public_key(&next).unwrap()
        );
        assert_eq!(
            template_hash(&first).unwrap(),
            template_hash(&next).unwrap()
        );
        assert_eq!(
            template_hash(&first).unwrap(),
            template_hash(&other_batcher).unwrap()
        );
    }

    #[test]
    fn published_template_hash_matches_the_covenant_source() {
        let compiled = veritas::hex::encode(&template_hash(&compile(&genesis()).unwrap()).unwrap());
        assert_eq!(
            TEMPLATE_HASH_FILE.trim(),
            compiled,
            "covenants/anchor.template-hash.txt is stale; regenerate with `cargo run -p veritas-anchor --bin anchor-abi`"
        );
    }

    #[test]
    fn scheme_bytes_are_the_canonical_kcc2_values() {
        assert_eq!(AuthorityScheme::P2pkSchnorr.byte(), 0x00);
        assert_eq!(AuthorityScheme::CovenantId.byte(), 0x04);
        for byte in [0x01, 0x02, 0x03, 0x05, 0x7f, 0x80] {
            assert_eq!(AuthorityScheme::from_byte(byte), None);
        }
    }
}
