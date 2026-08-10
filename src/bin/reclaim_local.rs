use veritas::{DemoResult, LocalVoteFixture, print_tx_summary};

fn main() -> DemoResult<()> {
    let mut fixture = LocalVoteFixture::new()?;
    fixture.vote(202608)?;
    let transaction = fixture.reclaim(202609)?;

    println!("built VoteDeposit::reclaim for 202609");
    print_tx_summary(&transaction);
    println!("artifact: build/vote/artifact.json");
    Ok(())
}
