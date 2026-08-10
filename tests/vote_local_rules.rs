use veritas::{DemoResult, LocalVoteFixture};

#[test]
fn first_vote_for_wallet_and_month_succeeds() -> DemoResult<()> {
    let mut fixture = LocalVoteFixture::new()?;

    fixture.vote(202608)?;

    Ok(())
}

#[test]
fn second_vote_for_wallet_and_month_in_same_ballot_lineage_fails() -> DemoResult<()> {
    let mut fixture = LocalVoteFixture::new()?;
    fixture.vote(202608)?;

    assert!(fixture.vote(202608).is_err());
    Ok(())
}

#[test]
fn reclaim_in_vote_month_fails() -> DemoResult<()> {
    let mut fixture = LocalVoteFixture::new()?;
    fixture.vote(202608)?;

    assert!(fixture.reclaim(202608).is_err());
    Ok(())
}

#[test]
fn reclaim_in_unlock_window_succeeds() -> DemoResult<()> {
    let mut fixture = LocalVoteFixture::new()?;
    fixture.vote(202608)?;

    fixture.reclaim(202609)?;
    Ok(())
}

#[test]
fn abandon_is_rejected_before_grace_and_allowed_after_grace() -> DemoResult<()> {
    let mut fixture = LocalVoteFixture::new()?;
    fixture.vote(202608)?;

    assert!(fixture.abandon(202610).is_err());
    fixture.abandon(202611)?;
    Ok(())
}
