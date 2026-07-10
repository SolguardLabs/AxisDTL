use serde::Serialize;

use crate::{AccountId, Amount, AssetConfig, AxisError, AxisResult, Bps, Digest};

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ExecutionQuote {
    pub solver: AccountId,
    pub source_asset: crate::AssetId,
    pub target_asset: crate::AssetId,
    pub price_numerator: u128,
    pub price_denominator: u128,
    pub solver_fee_bps: Bps,
    pub quote_nonce: u64,
    pub expires_at_epoch: u64,
    pub venue_digest: Digest,
}

impl ExecutionQuote {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        solver: AccountId,
        source_asset: crate::AssetId,
        target_asset: crate::AssetId,
        price_numerator: u128,
        price_denominator: u128,
        solver_fee_bps: Bps,
        quote_nonce: u64,
        expires_at_epoch: u64,
        venue_digest: Digest,
    ) -> AxisResult<Self> {
        if price_numerator == 0 || price_denominator == 0 {
            return Err(AxisError::DivisionByZero);
        }
        Ok(Self {
            solver,
            source_asset,
            target_asset,
            price_numerator,
            price_denominator,
            solver_fee_bps,
            quote_nonce,
            expires_at_epoch,
            venue_digest,
        })
    }

    pub fn digest(self) -> AxisResult<Digest> {
        Digest::from_serializable("axis-execution-quote-v1", &self)
    }

    pub fn output_amount(
        self,
        source_amount: Amount,
        source: AssetConfig,
        target: AssetConfig,
    ) -> AxisResult<Amount> {
        if self.source_asset != source.id || self.target_asset != target.id {
            return Err(AxisError::Policy("quote asset mismatch".to_owned()));
        }

        let source_scale = 10u128
            .checked_pow(u32::from(source.decimals))
            .ok_or(AxisError::AmountOverflow)?;
        let target_scale = 10u128
            .checked_pow(u32::from(target.decimals))
            .ok_or(AxisError::AmountOverflow)?;

        source_amount
            .checked_mul(self.price_numerator)?
            .checked_mul(source_scale)?
            .checked_div(
                self.price_denominator
                    .checked_mul(target_scale)
                    .ok_or(AxisError::AmountOverflow)?,
            )
    }

    pub fn fee_amount(self, output: Amount) -> AxisResult<Amount> {
        output.checked_mul_bps(self.solver_fee_bps)
    }
}
