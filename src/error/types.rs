use thiserror::Error;

use crate::{AccountId, Amount, AssetId, Digest, OrderId, TxId};

pub type AxisResult<T> = Result<T, AxisError>;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum AxisError {
    #[error("amount overflow")]
    AmountOverflow,
    #[error("amount underflow")]
    AmountUnderflow,
    #[error("division by zero")]
    DivisionByZero,
    #[error("zero amount")]
    ZeroAmount,
    #[error("basis points out of range: {0}")]
    BpsOutOfRange(u16),
    #[error("invalid decimals: {0}")]
    InvalidDecimals(u8),
    #[error("serialization error: {0}")]
    Serialization(String),
    #[error("signature error: {0}")]
    Signature(String),
    #[error("account already exists: {0}")]
    AccountAlreadyExists(AccountId),
    #[error("account not found: {0}")]
    AccountNotFound(AccountId),
    #[error("asset already exists: {0}")]
    AssetAlreadyExists(AssetId),
    #[error("asset not found: {0}")]
    AssetNotFound(AssetId),
    #[error("order already settled: {0}")]
    OrderSettled(OrderId),
    #[error("duplicate transaction: {0}")]
    DuplicateTransaction(TxId),
    #[error(
        "insufficient funds for {account} on {asset}: available {available}, required {required}"
    )]
    InsufficientFunds {
        account: AccountId,
        asset: AssetId,
        available: Amount,
        required: Amount,
    },
    #[error("nonce mismatch for {account}: expected {expected}, received {received}")]
    NonceMismatch {
        account: AccountId,
        expected: u64,
        received: u64,
    },
    #[error("unauthorized swap signer: expected {expected}, received {received}")]
    UnauthorizedSwapSigner {
        expected: AccountId,
        received: AccountId,
    },
    #[error("unauthorized settlement signer: expected {expected}, received {received}")]
    UnauthorizedSettlementSigner {
        expected: AccountId,
        received: AccountId,
    },
    #[error("quote digest mismatch for {order_id}: expected {expected}, received {received}")]
    QuoteDigestMismatch {
        order_id: OrderId,
        expected: Digest,
        received: Digest,
    },
    #[error("nonce overflow")]
    NonceOverflow,
    #[error("policy violation: {0}")]
    Policy(String),
    #[error("conservation error for {asset}: expected {expected}, observed {observed}")]
    Conservation {
        asset: AssetId,
        expected: Amount,
        observed: Amount,
    },
}
