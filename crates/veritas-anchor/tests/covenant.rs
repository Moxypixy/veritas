//! Runs the compiled anchor covenant in rusty-kaspa's script engine.

use kaspa_consensus_core::{
    Hash,
    hashing::{
        sighash::{SigHashReusedValuesUnsync, calc_schnorr_signature_hash},
        sighash_type::SIG_HASH_ALL,
    },
    tx::{
        CovenantBinding, PopulatedTransaction, ScriptPublicKey, Transaction, TransactionId,
        TransactionInput, TransactionOutpoint, TransactionOutput, UtxoEntry, VerifiableTransaction,
    },
};
use kaspa_txscript::{
    EngineCtx, EngineFlags, TxScriptEngine, caches::Cache, covenants::CovenantsContext,
};
use kaspa_txscript_errors::TxScriptError;
use secp256k1::{Keypair, Message, SECP256K1, Secp256k1, SecretKey};
use silverscript_abi::SilAbiArtifact;
use veritas_anchor::{
    AnchorState, AuthorityScheme, append_sig_script, compile, compile_raw, script_public_key,
};

const COVENANT: Hash = Hash::from_bytes(*b"VERITAS-ANCHOR-COVENANT-ID-00000");
const GOVERNANCE: Hash = Hash::from_bytes(*b"GOVERNANCE-MULTISIG-COVENANT-ID!");
const VALUE: u64 = 10_000_000;

fn keypair(byte: u8) -> Keypair {
    Keypair::from_secret_key(
        &Secp256k1::new(),
        &SecretKey::from_slice(&[byte; 32]).unwrap(),
    )
}

fn x_only(keypair: &Keypair) -> [u8; 32] {
    keypair.x_only_public_key().0.serialize()
}

fn key_anchor(keypair: &Keypair) -> AnchorState {
    AnchorState::genesis(x_only(keypair), AuthorityScheme::P2pkSchnorr)
}

fn utxo(artifact: &SilAbiArtifact) -> UtxoEntry {
    UtxoEntry::new(
        VALUE,
        script_public_key(artifact).unwrap(),
        0,
        false,
        Some(COVENANT),
    )
}

fn successor(artifact: &SilAbiArtifact) -> TransactionOutput {
    TransactionOutput {
        value: VALUE - 5_000,
        script_public_key: script_public_key(artifact).unwrap(),
        covenant: Some(CovenantBinding {
            authorizing_input: 0,
            covenant_id: COVENANT,
        }),
    }
}

fn input(index: u32, signature_script: Vec<u8>) -> TransactionInput {
    TransactionInput::new(
        TransactionOutpoint {
            transaction_id: TransactionId::from_bytes([index as u8 + 1; 32]),
            index,
        },
        signature_script,
        0,
        0,
    )
}

/// How the batcher authority approves the append (KCC-2 witness layouts).
enum Approval<'a> {
    /// `p2pk-schnorr/v1`: 65-byte SIGHASH_ALL signature by this key.
    Key(&'a Keypair),
    /// `covenant-id/v1`: spend an input of this lineage; empty witness.
    CovenantInput(Hash),
    /// `covenant-id/v1` claimed, but no such input is spent.
    Nothing,
    /// An arbitrary witness, to probe malformed layouts.
    Witness(Vec<u8>),
}

/// Builds an `append` spend of `current` with the given outputs and approval.
fn append_tx(
    current: &SilAbiArtifact,
    batch_root: &[u8; 32],
    approval: Approval,
    outputs: Vec<TransactionOutput>,
) -> (Transaction, Vec<UtxoEntry>) {
    let mut entries = vec![utxo(current)];
    let mut inputs = vec![input(0, vec![])];
    if let Approval::CovenantInput(authority) = approval {
        // Any input of the authority lineage; its own program is not run here.
        entries.push(UtxoEntry::new(
            1_000,
            ScriptPublicKey::new(0, vec![0x51].into()),
            0,
            false,
            Some(authority),
        ));
        inputs.push(input(1, vec![]));
    }
    let mut tx = Transaction::new(1, inputs, outputs, 0, Default::default(), 0, vec![]);

    let witness = match approval {
        Approval::Key(signer) => {
            let reused = SigHashReusedValuesUnsync::new();
            let populated = PopulatedTransaction::new(&tx, entries.clone());
            let sighash = calc_schnorr_signature_hash(&populated, 0, SIG_HASH_ALL, &reused);
            let message = Message::from_digest_slice(sighash.as_bytes().as_slice()).unwrap();
            let mut signature = SECP256K1
                .sign_schnorr_no_aux_rand(&message, signer)
                .as_ref()
                .to_vec();
            signature.push(SIG_HASH_ALL.to_u8());
            signature
        }
        Approval::CovenantInput(_) | Approval::Nothing => vec![],
        Approval::Witness(bytes) => bytes,
    };
    tx.inputs[0].signature_script = append_sig_script(current, batch_root, &witness).unwrap();
    tx.finalize();
    (tx, entries)
}

fn execute(tx: Transaction, entries: Vec<UtxoEntry>) -> Result<(), TxScriptError> {
    let reused = SigHashReusedValuesUnsync::new();
    let cache = Cache::new(1_000);
    let input = tx.inputs[0].clone();
    let populated = PopulatedTransaction::new(&tx, entries);
    let covenants = CovenantsContext::from_tx(&populated).map_err(TxScriptError::from)?;
    let utxo = populated.utxo(0).unwrap();
    let mut vm = TxScriptEngine::from_transaction_input(
        &populated,
        &input,
        0,
        utxo,
        EngineCtx::new(&cache)
            .with_reused(&reused)
            .with_covenants_ctx(&covenants),
        EngineFlags {
            covenants_enabled: true,
            ..Default::default()
        },
    );
    vm.execute()
}

#[test]
fn honest_append_is_accepted() {
    let batcher = keypair(7);
    let state = key_anchor(&batcher);
    let root = [0x42; 32];
    let current = compile(&state).unwrap();
    let next = compile(&state.after(&root)).unwrap();

    let (tx, entries) = append_tx(
        &current,
        &root,
        Approval::Key(&batcher),
        vec![successor(&next)],
    );
    execute(tx, entries).expect("append with the right accumulator must pass");
}

#[test]
fn script_hash_matches_the_off_chain_accumulator_over_three_batches() {
    let batcher = keypair(7);
    let roots = [[1; 32], [2; 32], [3; 32]];
    let mut state = key_anchor(&batcher);
    for root in &roots {
        let current = compile(&state).unwrap();
        let next_state = state.after(root);
        let (tx, entries) = append_tx(
            &current,
            root,
            Approval::Key(&batcher),
            vec![successor(&compile(&next_state).unwrap())],
        );
        execute(tx, entries).expect("each append in the chain must pass");
        state = next_state;
    }
    assert_eq!(state.acc, veritas::fold(&roots));
    assert_eq!(state.count, 3);
}

#[test]
fn rewriting_history_is_rejected() {
    let batcher = keypair(7);
    let state = key_anchor(&batcher).after(&[1; 32]);
    let root = [2; 32];
    let current = compile(&state).unwrap();
    // Try to replace the accumulator with one that drops the first batch.
    let forged = AnchorState {
        acc: veritas::fold(&[root]),
        ..state.after(&root)
    };

    let (tx, entries) = append_tx(
        &current,
        &root,
        Approval::Key(&batcher),
        vec![successor(&compile(&forged).unwrap())],
    );
    assert!(
        execute(tx, entries).is_err(),
        "a successor that is not an append must fail"
    );
}

#[test]
fn another_key_cannot_append() {
    let batcher = keypair(7);
    let state = key_anchor(&batcher);
    let root = [0x42; 32];
    let current = compile(&state).unwrap();
    let next = compile(&state.after(&root)).unwrap();

    let (tx, entries) = append_tx(
        &current,
        &root,
        Approval::Key(&keypair(9)),
        vec![successor(&next)],
    );
    assert!(
        execute(tx, entries).is_err(),
        "only the batcher key may append"
    );
}

#[test]
fn malformed_witnesses_are_rejected() {
    let batcher = keypair(7);
    let state = key_anchor(&batcher);
    let root = [0x42; 32];
    let current = compile(&state).unwrap();
    let next = compile(&state.after(&root)).unwrap();

    for witness in [vec![], vec![0; 64], vec![0; 66]] {
        let len = witness.len();
        let (tx, entries) = append_tx(
            &current,
            &root,
            Approval::Witness(witness),
            vec![successor(&next)],
        );
        assert!(
            execute(tx, entries).is_err(),
            "{len}-byte witness must fail"
        );
    }
}

#[test]
fn batcher_cannot_hand_over_the_anchor() {
    let batcher = keypair(7);
    let state = key_anchor(&batcher);
    let root = [0x42; 32];
    let current = compile(&state).unwrap();
    let handovers = [
        AnchorState {
            batcher: x_only(&keypair(9)),
            ..state.after(&root)
        },
        AnchorState {
            batcher_scheme: AuthorityScheme::CovenantId,
            ..state.after(&root)
        },
    ];

    for handover in handovers {
        let (tx, entries) = append_tx(
            &current,
            &root,
            Approval::Key(&batcher),
            vec![successor(&compile(&handover).unwrap())],
        );
        assert!(
            execute(tx, entries).is_err(),
            "the batcher authority is fixed"
        );
    }
}

#[test]
fn anchor_cannot_fork_or_vanish() {
    let batcher = keypair(7);
    let state = key_anchor(&batcher);
    let root = [0x42; 32];
    let current = compile(&state).unwrap();
    let next = compile(&state.after(&root)).unwrap();

    let (tx, entries) = append_tx(
        &current,
        &root,
        Approval::Key(&batcher),
        vec![successor(&next), successor(&next)],
    );
    assert!(execute(tx, entries).is_err(), "two successors must fail");

    let (tx, entries) = append_tx(&current, &root, Approval::Key(&batcher), vec![]);
    assert!(execute(tx, entries).is_err(), "no successor must fail");
}

#[test]
fn covenant_id_authority_appends_when_its_lineage_participates() {
    let state = AnchorState::genesis(GOVERNANCE.as_bytes(), AuthorityScheme::CovenantId);
    let root = [0x42; 32];
    let current = compile(&state).unwrap();
    let next = compile(&state.after(&root)).unwrap();

    let (tx, entries) = append_tx(
        &current,
        &root,
        Approval::CovenantInput(GOVERNANCE),
        vec![successor(&next)],
    );
    execute(tx, entries).expect("governance covenant participating must authorise the append");

    let (tx, entries) = append_tx(&current, &root, Approval::Nothing, vec![successor(&next)]);
    assert!(
        execute(tx, entries).is_err(),
        "without the governance input the append must fail"
    );

    let other = Hash::from_bytes([0x33; 32]);
    let (tx, entries) = append_tx(
        &current,
        &root,
        Approval::CovenantInput(other),
        vec![successor(&next)],
    );
    assert!(
        execute(tx, entries).is_err(),
        "another lineage is not the authority"
    );

    let (tx, entries) = append_tx(
        &current,
        &root,
        Approval::Witness(vec![0; 65]),
        vec![successor(&next)],
    );
    assert!(
        execute(tx, entries).is_err(),
        "covenant-id/v1 requires an empty witness"
    );
}

#[test]
fn unsupported_scheme_bytes_cannot_append() {
    let batcher = keypair(7);
    for scheme in [0x01u8, 0x03, 0x05, 0x80] {
        let current = compile_raw(x_only(&batcher), scheme, veritas::GENESIS_ACC, 0).unwrap();
        let root = [0x42; 32];
        let next = compile_raw(
            x_only(&batcher),
            scheme,
            veritas::next_acc(&veritas::GENESIS_ACC, &root),
            1,
        )
        .unwrap();
        let (tx, entries) = append_tx(
            &current,
            &root,
            Approval::Key(&batcher),
            vec![successor(&next)],
        );
        assert!(
            execute(tx, entries).is_err(),
            "scheme {scheme:#04x} must be rejected"
        );
    }
}
