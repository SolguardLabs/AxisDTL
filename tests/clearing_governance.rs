use std::collections::BTreeMap;

use axis_dtl::{
    AccountId, ActionKind, Amount, AssetId, Bps, ClearingEngine, ClearingLimits,
    ClearingObligation, ControlAction, ControlCommittee, ControlVote, ControlVoteKind,
    ControlVoteOutcome, Digest, KeyPair, SignedControlAction, SignedControlVote,
};

fn account(value: u8) -> AccountId {
    AccountId::from_bytes([value; 32])
}

fn asset(label: &str) -> AssetId {
    AssetId::derive(label, 6)
}

fn obligation(
    debtor: AccountId,
    creditor: AccountId,
    asset: AssetId,
    units: u128,
    window: u64,
    reference: u8,
) -> ClearingObligation {
    ClearingObligation::new(
        debtor,
        creditor,
        asset,
        Amount::new(units).expect("amount"),
        window,
        Digest::from_bytes([reference; 32]),
    )
    .expect("obligation")
}

#[test]
fn multilateral_cycle_compresses_gross_obligations() {
    let usd = asset("AXUSD");
    let limits = ClearingLimits::new(16, 8, Bps::new(1_000).unwrap()).unwrap();
    let engine = ClearingEngine::new(limits);
    let obligations = vec![
        obligation(account(1), account(2), usd, 100, 70, 1),
        obligation(account(2), account(3), usd, 70, 70, 2),
        obligation(account(3), account(1), usd, 40, 70, 3),
    ];
    let reserves = BTreeMap::from([(usd, Amount::new(65).unwrap())]);

    let cycle = engine.preview(70, &obligations, &reserves).unwrap();
    let summary = cycle.asset(usd).unwrap();

    assert_eq!(summary.gross_obligations.units(), 210);
    assert_eq!(summary.net_payable.units(), 60);
    assert_eq!(summary.compressed_amount.units(), 150);
    assert_eq!(summary.compression_bps.units(), 7_142);
    assert_eq!(summary.required_reserve.units(), 66);
    assert_eq!(summary.reserve_shortfall.units(), 1);
    assert!(!cycle.fully_reserved());
}

#[test]
fn clearing_keeps_assets_and_positions_separate() {
    let usd = asset("AXUSD");
    let eur = asset("AXEUR");
    let engine = ClearingEngine::new(ClearingLimits::new(16, 8, Bps::new(0).unwrap()).unwrap());
    let obligations = vec![
        obligation(account(1), account(2), usd, 30, 71, 4),
        obligation(account(2), account(1), eur, 20, 71, 5),
    ];
    let reserves = BTreeMap::from([
        (usd, Amount::new(30).unwrap()),
        (eur, Amount::new(20).unwrap()),
    ]);

    let cycle = engine.preview(71, &obligations, &reserves).unwrap();

    assert_eq!(cycle.assets.len(), 2);
    assert_eq!(cycle.position_count, 4);
    assert_eq!(cycle.asset(usd).unwrap().net_payable.units(), 30);
    assert_eq!(cycle.asset(eur).unwrap().net_payable.units(), 20);
    assert!(cycle.fully_reserved());
}

#[test]
fn clearing_rejects_duplicate_references_and_wrong_windows() {
    let usd = asset("AXUSD");
    let engine = ClearingEngine::new(ClearingLimits::new(4, 4, Bps::new(0).unwrap()).unwrap());
    let duplicate = vec![
        obligation(account(1), account(2), usd, 10, 72, 6),
        obligation(account(2), account(3), usd, 5, 72, 6),
    ];
    assert!(engine.preview(72, &duplicate, &BTreeMap::new()).is_err());

    let wrong_window = vec![obligation(account(1), account(2), usd, 10, 73, 7)];
    assert!(engine.preview(72, &wrong_window, &BTreeMap::new()).is_err());
}

fn committee() -> (ControlCommittee, KeyPair, KeyPair, KeyPair) {
    let first = KeyPair::from_seed([11; 32]);
    let second = KeyPair::from_seed([22; 32]);
    let third = KeyPair::from_seed([33; 32]);
    let mut committee = ControlCommittee::new(2).unwrap();
    committee
        .register_reviewer(first.public_identity())
        .unwrap();
    committee
        .register_reviewer(second.public_identity())
        .unwrap();
    committee
        .register_reviewer(third.public_identity())
        .unwrap();
    (committee, first, second, third)
}

fn action(key: &KeyPair, nonce: u64, marker: u8) -> SignedControlAction {
    let action = ControlAction::new(
        ActionKind::RiskLimits,
        Digest::from_bytes([marker; 32]),
        key.public_identity().account,
        nonce,
        100,
        120,
    )
    .unwrap();
    SignedControlAction::sign(action, key).unwrap()
}

fn vote(key: &KeyPair, digest: Digest, nonce: u64, decision: ControlVoteKind) -> SignedControlVote {
    SignedControlVote::sign(
        ControlVote {
            action_digest: digest,
            reviewer: key.public_identity().account,
            reviewer_nonce: nonce,
            decision,
        },
        key,
    )
    .unwrap()
}

#[test]
fn signed_quorum_and_timelock_gate_control_execution() {
    let (mut committee, first, second, _) = committee();
    let digest = committee.submit(&action(&first, 0, 41)).unwrap();
    let outcome = committee
        .vote(&vote(&second, digest, 0, ControlVoteKind::Approve))
        .unwrap();

    assert_eq!(outcome, ControlVoteOutcome::Approved);
    assert!(committee.execute(digest, 99).is_err());
    let executed = committee.execute(digest, 100).unwrap();
    assert_eq!(executed.kind, ActionKind::RiskLimits);
    assert!(committee.was_executed(digest));
}

#[test]
fn signatures_nonces_and_unique_votes_are_enforced() {
    let (mut committee, first, second, _) = committee();
    let mut tampered = action(&first, 0, 42);
    tampered.action.earliest_epoch = 101;
    assert!(committee.submit(&tampered).is_err());

    let digest = committee.submit(&action(&first, 0, 43)).unwrap();
    let approval = vote(&second, digest, 0, ControlVoteKind::Approve);
    committee.vote(&approval).unwrap();
    assert!(committee.vote(&approval).is_err());
    assert!(committee.submit(&action(&first, 0, 44)).is_err());
}

#[test]
fn cancellation_requires_independent_quorum() {
    let (mut committee, first, second, third) = committee();
    let digest = committee.submit(&action(&first, 0, 45)).unwrap();

    assert_eq!(
        committee
            .vote(&vote(&second, digest, 0, ControlVoteKind::Cancel))
            .unwrap(),
        ControlVoteOutcome::Pending
    );
    assert_eq!(
        committee
            .vote(&vote(&third, digest, 0, ControlVoteKind::Cancel))
            .unwrap(),
        ControlVoteOutcome::Cancelled
    );
    assert!(committee.pending(digest).is_none());
    assert!(committee.execute(digest, 100).is_err());
}
