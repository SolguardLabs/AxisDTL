mod amount;
mod codec;
mod crypto;
mod custody;
mod error;
mod ids;
mod ledger;
mod market;
mod oracle;
mod policy;
mod routing;
mod runtime;

pub use amount::{Amount, Bps};
pub use codec::canonical_bytes;
pub use crypto::{KeyPair, PublicIdentity, SignatureBytes, verify_signature};
pub use custody::{CustodyState, MarginAccount, MarginMode, TreasuryPolicy, VaultConfig};
pub use error::{AxisError, AxisResult};
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

fn main() {
    if let Err(error) = runtime::run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
