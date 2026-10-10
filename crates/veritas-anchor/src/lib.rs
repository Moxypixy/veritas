//! The Veritas anchor covenant, compiled with Silverscript.
//!
//! One covenant UTXO carries `(batcher, acc, count)`. Each `append` spends it
//! and must create exactly one successor whose `acc` is
//! `blake2b(acc || batch_root)` (see [`veritas::accumulator::next_acc`]).
//! Because Silverscript bakes the state into the P2SH redeem script, every
//! state has its own compiled script; [`compile`] produces it.

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

/// The covenant's state fields.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AnchorState {
    /// x-only public key allowed to append.
    pub batcher: [u8; 32],
    /// Accumulator over every anchored batch root.
    pub acc: [u8; 32],
    /// Number of batches anchored so far.
    pub count: i64,
}

impl AnchorState {
    pub fn genesis(batcher: [u8; 32]) -> Self {
        Self {
            batcher,
            acc: veritas::GENESIS_ACC,
            count: 0,
        }
    }

    /// The only successor `append(batch_root)` accepts.
    pub fn after(&self, batch_root: &[u8; 32]) -> Self {
        Self {
            batcher: self.batcher,
            acc: veritas::next_acc(&self.acc, batch_root),
            count: self.count + 1,
        }
    }
}

/// Compiles the covenant for one concrete state.
pub fn compile(state: &AnchorState) -> Result<SilAbiArtifact, Box<CompilerError>> {
    let args: [ArtifactValue; 3] = [
        state.batcher.to_vec().into(),
        state.acc.to_vec().into(),
        state.count.into(),
    ];
    compile_to_sil_abi_artifact_with_options(ANCHOR_SOURCE, &args, CompileOptions::default())
        .map_err(Box::new)
}

/// Redeem script bytes of a compiled state.
pub fn redeem_script(artifact: &SilAbiArtifact) -> AnchorResult<&[u8]> {
    let contract = artifact
        .contracts
        .get(CONTRACT_NAME)
        .ok_or("artifact has no VeritasAnchor contract")?;
    Ok(&contract.compiled.bytecode)
}

/// P2SH script public key that locks a UTXO to this state.
pub fn script_public_key(artifact: &SilAbiArtifact) -> AnchorResult<ScriptPublicKey> {
    Ok(pay_to_script_hash_script(redeem_script(artifact)?))
}

/// Signature script for `append(batch_root, batcher_sig)` spending `current`.
///
/// `batcher_sig` is a 65-byte Schnorr signature (64 bytes + sighash type) over
/// the spending transaction.
pub fn append_sig_script(
    current: &SilAbiArtifact,
    batch_root: &[u8; 32],
    batcher_sig: &[u8],
) -> AnchorResult<Vec<u8>> {
    let args: [ArtifactValue; 2] = [batch_root.to_vec().into(), batcher_sig.to_vec().into()];
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

    #[test]
    fn compiles_and_exposes_append() {
        let artifact = compile(&AnchorState::genesis([0xaa; 32])).unwrap();
        let contract = &artifact.contracts[CONTRACT_NAME];
        assert!(
            contract.entries.contains_key("append"),
            "entries: {:?}",
            contract.entries.keys()
        );
    }

    #[test]
    fn each_state_has_its_own_script() {
        let genesis = AnchorState::genesis([0xaa; 32]);
        let next = genesis.after(&[1; 32]);
        assert_ne!(
            script_public_key(&compile(&genesis).unwrap()).unwrap(),
            script_public_key(&compile(&next).unwrap()).unwrap()
        );
    }
}
