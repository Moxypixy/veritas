use argent::build_file;
use argent_runtime::{EntryCall, TxBuilder, TxContext, args, state};
use kaspa_consensus_core::{Hash, tx::CovenantBinding};
use veritas::{DemoResult, demo_outpoint, print_tx_summary};

fn main() -> DemoResult<()> {
    let artifact = build_file("ag/counter.ag", "build/counter")?;
    let builder = TxBuilder::new(&artifact)?;

    // prep data
    let count = 2i64;
    let delta = 3i64;
    let counter_outpoint = demo_outpoint(0x11, 0);

    // build state objects
    let before = state! {
        count: count,
    };
    let after = state! {
        count: count + delta,
    };
    let value = 1_000;
    let covenant_id = Hash::from_bytes([0x42; 32]);

    let counter_utxo = builder.covenant_utxo(
        "Counter",
        before.clone(),
        value,
        0,
        false,
        Some(covenant_id),
    )?;
    let context = TxContext::new()
        .actor_input(
            "Counter",
            before,
            EntryCall::new("bump").args(args![delta]),
            counter_outpoint,
            counter_utxo,
            0,
        )
        .actor_output(
            "Counter",
            after,
            CovenantBinding::new(0, covenant_id),
            value,
        );

    let tx = builder.build(&context)?;

    println!("built Counter::bump");
    print_tx_summary(&tx);
    println!("artifact: build/counter/artifact.json");
    Ok(())
}
