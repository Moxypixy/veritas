use veritas::{DemoResult, LocalVoteFixture, print_tx_summary};

fn main() -> DemoResult<()> {
    let mut fixture = LocalVoteFixture::new()?;
    let transaction = fixture.vote(202608)?;

    println!("built VoterBallot::vote for 202608");
    print_tx_summary(&transaction);
    println!("artifact: build/vote/artifact.json");
    Ok(())
}
