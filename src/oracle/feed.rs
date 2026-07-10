use serde::Serialize;

use crate::{AccountId, AssetId, AxisError, AxisResult, Bps, Digest};

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize)]
pub struct PriceLevel {
    pub base_asset: AssetId,
    pub quote_asset: AssetId,
    pub price_numerator: u128,
    pub price_denominator: u128,
    pub confidence_bps: Bps,
    pub updated_at_epoch: u64,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize)]
pub struct OracleObservation {
    pub feed_id: Digest,
    pub publisher: AccountId,
    pub level: PriceLevel,
}

impl PriceLevel {
    pub fn new(
        base_asset: AssetId,
        quote_asset: AssetId,
        price_numerator: u128,
        price_denominator: u128,
        confidence_bps: Bps,
        updated_at_epoch: u64,
    ) -> AxisResult<Self> {
        if price_numerator == 0 || price_denominator == 0 {
            return Err(AxisError::DivisionByZero);
        }
        Ok(Self {
            base_asset,
            quote_asset,
            price_numerator,
            price_denominator,
            confidence_bps,
            updated_at_epoch,
        })
    }

    pub fn market_digest(self) -> Digest {
        Digest::from_parts(
            "axis-oracle-market-v1",
            &[&self.base_asset.bytes(), &self.quote_asset.bytes()],
        )
    }

    pub fn deviation_bps(self, numerator: u128, denominator: u128) -> AxisResult<Bps> {
        if numerator == 0 || denominator == 0 {
            return Err(AxisError::DivisionByZero);
        }
        let lhs = numerator
            .checked_mul(self.price_denominator)
            .ok_or(AxisError::AmountOverflow)?;
        let rhs = self
            .price_numerator
            .checked_mul(denominator)
            .ok_or(AxisError::AmountOverflow)?;
        let delta = lhs.abs_diff(rhs);
        let reference = rhs.max(1);
        let units = delta
            .checked_mul(10_000)
            .ok_or(AxisError::AmountOverflow)?
            .checked_div(reference)
            .ok_or(AxisError::DivisionByZero)?;
        Bps::new(units.min(10_000) as u16)
    }
}
