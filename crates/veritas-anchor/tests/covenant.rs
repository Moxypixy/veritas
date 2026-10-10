//! Runs the compiled anchor covenant in rusty-kaspa's script engine.

use kaspa_consensus_core::{
    Hash,
    hashing::{
        sighash::{SigHashReusedValuesUnsync, calc_schnorr_signature_hash},
        sighash_type::SIG_HASH_ALL,
    },
    tx::{
        CovenantBinding, PopulatedTransaction, Transaction, TransactionId, TransactionInput,
        TransactionOutpoint, TransactionOutput, UtxoEntry, VerifiableTransaction,
    },
};
use kaspa_txscript::{
    EngineCtx, EngineFlags, TxScriptEngine, caches::Cache, covenants::CovenantsContext,
};
use kaspa_txscript_errors::TxScriptError;
use secp256k1::{Keypair, Message, SECP256K1, Secp256k1, SecretKey};
use silverscript_abi::SilAbiArtifact;
use veritas_anchor::{AnchorState, append_sig_script, compile, script_public_key};

const COVENANT: Hash = Hash::from_bytes(*b"VERITAS-ANCHOR-COVENANT-ID-00000");
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

fn input(signature_script: Vec<u8>) -> TransactionInput {
    TransactionInput::new(
        TransactionOutpoint {
            transaction_id: TransactionId::from_bytes([1; 32]),
            index: 0,
        },
        signature_script,
        0,
        0,
    )
}

/// Builds and signs an `append` spend of `current`, with the given outputs.
fn append_tx(
    current: &SilAbiArtifact,
    batch_root: &[u8; 32],
    signer: &Keypair,
    outputs: Vec<TransactionOutput>,
) -> (Transaction, Vec<UtxoEntry>) {
    let entries = vec![utxo(current)];
    let placeholder = append_sig_script(current, batch_root, &[0; 65]).unwrap();
    let mut tx = Transaction::new(
        1,
        vec![input(placeholder)],
        outputs,
        0,
        Default::default(),
        0,
        vec![],
    );

    let reused = SigHashReusedValuesUnsync::new();
    let populated = PopulatedTransaction::new(&tx, entries.clone());
    let sighash = calc_schnorr_signature_hash(&populated, 0, SIG_HASH_ALL, &reused);
    let message = Message::from_digest_slice(sighash.as_bytes().as_slice()).unwrap();
    let mut signature = SECP256K1
        .sign_schnorr_no_aux_rand(&message, signer)
        .as_ref()
        .to_vec();
    signature.push(SIG_HASH_ALL.to_u8());

    tx.inputs[0].signature_script = append_sig_script(current, batch_root, &signature).unwrap();
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
    let state = AnchorState::genesis(x_only(&batcher));
    let root = [0x42; 32];
    let current = compile(&state).unwrap();
    let next = compile(&state.after(&root)).unwrap();

    let (tx, entries) = append_tx(&current, &root, &batcher, vec![successor(&next)]);
    execute(tx, entries).expect("append with the right accumulator must pass");
}

#[test]
fn script_hash_matches_the_off_chain_accumulator_over_three_batches() {
    let batcher = keypair(7);
    let roots = [[1; 32], [2; 32], [3; 32]];
    let mut state = AnchorState::genesis(x_only(&batcher));
    for root in &roots {
        let current = compile(&state).unwrap();
        let next_state = state.after(root);
        let (tx, entries) = append_tx(
            &current,
            root,
            &batcher,
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
    let state = AnchorState::genesis(x_only(&batcher)).after(&[1; 32]);
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
        &batcher,
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
    let intruder = keypair(9);
    let state = AnchorState::genesis(x_only(&batcher));
    let root = [0x42; 32];
    let current = compile(&state).unwrap();
    let next = compile(&state.after(&root)).unwrap();

    let (tx, entries) = append_tx(&current, &root, &intruder, vec![successor(&next)]);
    assert!(
        execute(tx, entries).is_err(),
        "only the batcher key may append"
    );
}

#[test]
fn batcher_cannot_hand_over_the_anchor() {
    let batcher = keypair(7);
    let state = AnchorState::genesis(x_only(&batcher));
    let root = [0x42; 32];
    let current = compile(&state).unwrap();
    let handover = AnchorState {
        batcher: x_only(&keypair(9)),
        ..state.after(&root)
    };

    let (tx, entries) = append_tx(
        &current,
        &root,
        &batcher,
        vec![successor(&compile(&handover).unwrap())],
    );
    assert!(
        execute(tx, entries).is_err(),
        "the batcher key is fixed for v0"
    );
}

#[test]
fn anchor_cannot_fork_or_vanish() {
    let batcher = keypair(7);
    let state = AnchorState::genesis(x_only(&batcher));
    let root = [0x42; 32];
    let current = compile(&state).unwrap();
    let next = compile(&state.after(&root)).unwrap();

    let (tx, entries) = append_tx(
        &current,
        &root,
        &batcher,
        vec![successor(&next), successor(&next)],
    );
    assert!(execute(tx, entries).is_err(), "two successors must fail");

    let (tx, entries) = append_tx(&current, &root, &batcher, vec![]);
    assert!(execute(tx, entries).is_err(), "no successor must fail");
}
