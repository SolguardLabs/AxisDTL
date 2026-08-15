mod amount;
mod clearing;
mod codec;
mod crypto;
mod custody;
mod error;
mod governance;
mod ids;
mod ledger;
mod market;
mod oracle;
mod policy;
mod routing;
mod runtime;

pub use amount::{Amount, Bps};
pub use clearing::{
    AssetClearingSummary, ClearingCycle, ClearingEngine, ClearingLimits, ClearingObligation,
    NetPosition,
};
pub use codec::canonical_bytes;
pub use crypto::{KeyPair, PublicIdentity, SignatureBytes, verify_signature};
pub use custody::{CustodyState, MarginAccount, MarginMode, TreasuryPolicy, VaultConfig};
pub use error::{AxisError, AxisResult};
pub use governance::{
    ActionKind, ControlAction, ControlCommittee, ControlVote, ControlVoteKind, ControlVoteOutcome,
    PendingControlAction, SignedControlAction, SignedControlVote,
};
pub use ids::{AccountId, AssetId, Digest, OrderId, TxId};
pub use ledger::{AccountState, AxisLedger, JournalEntry, JournalOp};
pub use market::{
    AssetConfig, ExecutionQuote, SettlementAuthorizationView, SettlementRequest, SignedSettlement,
    SignedSwap, SwapAuthorizationView, SwapTerms,
};
pub use oracle::{OracleObservation, OracleRegistry, PriceCheck, PriceLevel};
pub use policy::{AccountRiskProfile, ProtocolLimits, RiskDecision, RiskEngine, RiskTier};
pub use routing::{RouteBook, RouteLeg, RoutePlan, RouteQuality, VenueConfig, VenueKind};
pub use runtime::ScenarioReport;

pub fn run_cli() -> AxisResult<()> {
    runtime::run()
}
