//! Regenerates the published Program ABI and template hash of the anchor.
//!
//! ```text
//! cargo run -p veritas-anchor --bin anchor-abi
//! ```
//!
//! Writes `covenants/anchor.abi.json` (the Silverscript ABI artifact, compiled
//! for a placeholder genesis state) and `covenants/anchor.template-hash.txt`.
//! The template hash does not depend on the state, so wallets and indexers can
//! recognise every anchor program from it.

use std::{fs, path::PathBuf};

use veritas_anchor::{AnchorState, AuthorityScheme, compile, template_hash};

fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../covenants");
    let artifact = compile(&AnchorState::genesis([0; 32], AuthorityScheme::P2pkSchnorr))
        .map_err(|e| format!("{e:?}"))?;
    fs::write(
        dir.join("anchor.abi.json"),
        serde_json::to_string_pretty(&artifact)? + "\n",
    )?;
    let hash = veritas::hex::encode(&template_hash(&artifact)?);
    fs::write(dir.join("anchor.template-hash.txt"), format!("{hash}\n"))?;
    println!("template hash {hash}");
    Ok(())
}
