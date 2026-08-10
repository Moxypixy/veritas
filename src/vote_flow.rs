use argent::{artifact::Artifact, build_file};
use argent_runtime::{CovenantOutput, EntryCall, TxBuilder, TxContext, args, state};
use blake2b_simd::Params;
use kaspa_consensus_core::{
    Hash,
    tx::{CovenantBinding, Transaction, TransactionOutpoint, UtxoEntry},
};
use secp256k1::Keypair;
use std::sync::Mutex;

use crate::{
    DemoResult, VoteAnswer, commitment_hash, demo_funding_utxo, demo_keys, demo_outpoint,
    sign_input,
};

const DEPOSIT_SOMPI: u64 = 10_000_000;
const BALLOT_VALUE: u64 = 1;
static ARTIFACT_BUILD_LOCK: Mutex<()> = Mutex::new(());

struct ActorCoin {
    outpoint: TransactionOutpoint,
    utxo: UtxoEntry,
    covenant_id: Hash,
}

impl ActorCoin {
    fn from_output(transaction: &Transaction, output_index: u32) -> DemoResult<Self> {
        let output = CovenantOutput::from_tx(transaction, output_index)?;
        Ok(Self {
            outpoint: output.outpoint,
            utxo: output.utxo,
            covenant_id: output.covenant_id,
        })
    }
}

/// Builds the current Pulse artifact used by local runtime demonstrations.
pub fn load_pulse_artifact() -> DemoResult<Artifact> {
    let _guard = ARTIFACT_BUILD_LOCK
        .lock()
        .map_err(|_| std::io::Error::other("Pulse artifact build lock is poisoned"))?;
    Ok(build_file("ag/vote.ag", "build/vote")?)
}

/// Deterministic local state for exercising one fixture wallet's ballot lineage.
pub struct LocalVoteFixture {
    artifact: Artifact,
    voter_keypair: Keypair,
    voter_public_key: Vec<u8>,
    voter_id: Vec<u8>,
    ballot: ActorCoin,
    ballot_state: std::collections::BTreeMap<String, argent_runtime::ArtifactValue>,
    deposit: Option<ActorCoin>,
    deposit_state: Option<std::collections::BTreeMap<String, argent_runtime::ArtifactValue>>,
}

impl LocalVoteFixture {
    /// Opens a ballot for the deterministic fixture wallet.
    pub fn new() -> DemoResult<Self> {
        let artifact = load_pulse_artifact()?;
        let builder = TxBuilder::new(&artifact)?;
        let (voter_keypair, voter_public_key) = demo_keys(0x01);
        let voter_id = voter_identifier(&voter_public_key);
        let office_state = state! {
            ballots_opened: 0i64,
        };
        let office_id = Hash::from_bytes([0x11; 32]);
        let office_utxo = builder.covenant_utxo(
            "BallotOffice",
            office_state.clone(),
            BALLOT_VALUE,
            0,
            false,
            Some(office_id),
        )?;
        let next_office_state = state! {
            ballots_opened: 1i64,
        };
        let ballot_state = state! {
            voter: voter_id.clone(),
            last_month_yyyymm: 0i64,
        };
        let open_context = TxContext::new()
            .actor_input(
                "BallotOffice",
                office_state,
                EntryCall::new("open_ballot").args_with(|tx, input_idx| {
                    args![
                        sign_input(tx, input_idx, &voter_keypair),
                        voter_public_key.clone()
                    ]
                }),
                demo_outpoint(0x10, 0),
                office_utxo,
                0,
            )
            .input(
                demo_outpoint(0x12, 0),
                demo_funding_utxo(BALLOT_VALUE),
                Vec::new(),
                0,
            )
            .actor_output(
                "BallotOffice",
                next_office_state,
                CovenantBinding::new(0, office_id),
                BALLOT_VALUE,
            )
            .actor_output(
                "VoterBallot",
                ballot_state.clone(),
                CovenantBinding::new(0, office_id),
                BALLOT_VALUE,
            );
        let opening_transaction = builder.build(&open_context)?;
        drop(open_context);
        let ballot = ActorCoin::from_output(&opening_transaction, 1)?;

        Ok(Self {
            artifact,
            voter_keypair,
            voter_public_key,
            voter_id,
            ballot,
            ballot_state,
            deposit: None,
            deposit_state: None,
        })
    }

    /// Builds and validates a vote transaction for the fixture wallet.
    pub fn vote(&mut self, month_yyyymm: i64) -> DemoResult<Transaction> {
        let builder = TxBuilder::new(&self.artifact)?;
        let commitment = fixture_commitment()?;
        let next_ballot_state = state! {
            voter: self.voter_id.clone(),
            last_month_yyyymm: month_yyyymm,
        };
        let deposit_state = state! {
            voter: self.voter_id.clone(),
            month_yyyymm: month_yyyymm,
            unlock_yyyymm: add_months(month_yyyymm, 1),
            abandon_yyyymm: add_months(month_yyyymm, 3),
            commitment: commitment,
        };
        let vote_context = TxContext::new()
            .actor_input(
                "VoterBallot",
                self.ballot_state.clone(),
                EntryCall::new("vote").args_with(|tx, input_idx| {
                    args![
                        sign_input(tx, input_idx, &self.voter_keypair),
                        self.voter_public_key.clone(),
                        month_yyyymm,
                        month_yyyymm,
                        commitment,
                    ]
                }),
                self.ballot.outpoint,
                self.ballot.utxo.clone(),
                0,
            )
            .input(
                demo_outpoint(0x20, 0),
                demo_funding_utxo(DEPOSIT_SOMPI),
                Vec::new(),
                0,
            )
            .actor_output(
                "VoterBallot",
                next_ballot_state.clone(),
                CovenantBinding::new(0, self.ballot.covenant_id),
                BALLOT_VALUE,
            )
            .actor_output(
                "VoteDeposit",
                deposit_state.clone(),
                CovenantBinding::new(0, self.ballot.covenant_id),
                DEPOSIT_SOMPI,
            );
        let transaction = builder.build(&vote_context)?;
        self.ballot = ActorCoin::from_output(&transaction, 0)?;
        self.ballot_state = next_ballot_state;
        self.deposit = Some(ActorCoin::from_output(&transaction, 1)?);
        self.deposit_state = Some(deposit_state);
        Ok(transaction)
    }

    /// Builds and validates reclaim during the supplied fixture month.
    pub fn reclaim(&mut self, current_yyyymm: i64) -> DemoResult<Transaction> {
        let deposit = self.deposit.as_ref().ok_or("fixture has no vote deposit")?;
        let deposit_state = self
            .deposit_state
            .as_ref()
            .ok_or("fixture has no vote deposit state")?
            .clone();
        let builder = TxBuilder::new(&self.artifact)?;
        let reclaim_context = TxContext::new()
            .actor_input(
                "VoteDeposit",
                deposit_state,
                EntryCall::new("reclaim").args_with(|tx, input_idx| {
                    args![
                        sign_input(tx, input_idx, &self.voter_keypair),
                        self.voter_public_key.clone(),
                        current_yyyymm,
                    ]
                }),
                deposit.outpoint,
                deposit.utxo.clone(),
                0,
            )
            .actor_output(
                "WalletPurse",
                state! {
                    owner: self.voter_id.clone(),
                },
                CovenantBinding::new(0, deposit.covenant_id),
                DEPOSIT_SOMPI,
            );
        let transaction = builder.build(&reclaim_context)?;
        self.deposit = None;
        self.deposit_state = None;
        Ok(transaction)
    }

    /// Builds and validates the permissionless abandonment path.
    pub fn abandon(&mut self, current_yyyymm: i64) -> DemoResult<Transaction> {
        let deposit = self.deposit.as_ref().ok_or("fixture has no vote deposit")?;
        let deposit_state = self
            .deposit_state
            .as_ref()
            .ok_or("fixture has no vote deposit state")?
            .clone();
        let builder = TxBuilder::new(&self.artifact)?;
        let abandon_context = TxContext::new()
            .actor_input(
                "VoteDeposit",
                deposit_state,
                EntryCall::new("abandon").args(args![current_yyyymm]),
                deposit.outpoint,
                deposit.utxo.clone(),
                0,
            )
            .actor_output(
                "Burn",
                state! {
                    marker: 1i64,
                },
                CovenantBinding::new(0, deposit.covenant_id),
                DEPOSIT_SOMPI,
            );
        let transaction = builder.build(&abandon_context)?;
        self.deposit = None;
        self.deposit_state = None;
        Ok(transaction)
    }
}

fn voter_identifier(public_key: &[u8]) -> Vec<u8> {
    Params::new()
        .hash_length(32)
        .hash(public_key)
        .as_bytes()
        .to_vec()
}

fn fixture_commitment() -> DemoResult<[u8; 32]> {
    Ok(commitment_hash(&VoteAnswer {
        region_id: "euro-area".into(),
        necessities_pct: 42,
        employed: true,
        duration_months: 18,
        month_key: "2026-08".into(),
        wallet: "kaspatest:qqveritasfixturewallet1".into(),
        salt: [0x42; 16],
    })?)
}

fn add_months(yyyymm: i64, delta: i64) -> i64 {
    let year = yyyymm / 100;
    let month = yyyymm % 100;
    let zero_based = year * 12 + month - 1 + delta;
    zero_based / 12 * 100 + zero_based % 12 + 1
}
