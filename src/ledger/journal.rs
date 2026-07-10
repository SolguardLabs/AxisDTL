use serde::Serialize;

use crate::{
    AccountId, Amount, AssetId, Bps, Digest, OrderId, PriceCheck, RiskDecision, RouteQuality, TxId,
};

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct JournalEntry {
    pub sequence: u64,
    pub tx_id: TxId,
    pub op: JournalOp,
    pub state_digest: Digest,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum JournalOp {
    GenesisCredit {
        account: AccountId,
        asset: AssetId,
        amount: Amount,
    },
    SwapExecution {
        order_id: OrderId,
        payer: AccountId,
        receiver: AccountId,
        solver: AccountId,
        pool: AccountId,
        source_asset: AssetId,
        target_asset: AssetId,
        source_amount: Amount,
        gross_output: Amount,
        solver_fee: Amount,
        receiver_amount: Amount,
        route_quality: Box<RouteQuality>,
        price_check: Box<PriceCheck>,
        risk_decision: Box<RiskDecision>,
    },
    OraclePublished {
        feed_id: Digest,
        publisher: AccountId,
        market_digest: Digest,
    },
    VenueRegistered {
        venue_id: Digest,
        operator: AccountId,
    },
    RouteRegistered {
        route_id: Digest,
        quote_digest: Digest,
        leg_count: u8,
    },
    VaultRegistered {
        account: AccountId,
        custodian: AccountId,
        reserve_asset: AssetId,
        reserve_floor: Amount,
    },
    TreasuryConfigured {
        fee_recipient: AccountId,
        protocol_fee_bps: Bps,
    },
    RiskProfileUpdated {
        account: AccountId,
        enabled: bool,
    },
    MarginOpened {
        margin_id: Digest,
        owner: AccountId,
        collateral_asset: AssetId,
        collateral: Amount,
    },
}
