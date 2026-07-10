use std::collections::BTreeMap;

use serde::Serialize;

use crate::{
    AccountId, AssetId, AxisError, AxisResult, Bps, Digest, OracleObservation, PriceLevel,
};

#[derive(Clone, Debug, Serialize)]
pub struct OracleRegistry {
    stale_after_epochs: u64,
    latest_by_market: BTreeMap<Digest, OracleObservation>,
    publisher_weights: BTreeMap<AccountId, u16>,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize)]
pub struct PriceCheck {
    pub market_digest: Digest,
    pub observed_deviation_bps: Bps,
    pub accepted_deviation_bps: Bps,
    pub confidence_bps: Bps,
}

impl OracleRegistry {
    pub fn new(stale_after_epochs: u64) -> Self {
        Self {
            stale_after_epochs,
            latest_by_market: BTreeMap::new(),
            publisher_weights: BTreeMap::new(),
        }
    }

    pub fn register_publisher(&mut self, publisher: AccountId, weight: u16) -> AxisResult<()> {
        if weight == 0 {
            return Err(AxisError::Policy(
                "oracle publisher weight is zero".to_owned(),
            ));
        }
        self.publisher_weights.insert(publisher, weight);
        Ok(())
    }

    pub fn publish(
        &mut self,
        publisher: AccountId,
        level: PriceLevel,
    ) -> AxisResult<OracleObservation> {
        if !self.publisher_weights.contains_key(&publisher) {
            return Err(AxisError::Policy("unknown oracle publisher".to_owned()));
        }
        let feed_id = Digest::from_serializable("axis-oracle-observation-v1", &(publisher, level))?;
        let observation = OracleObservation {
            feed_id,
            publisher,
            level,
        };
        self.latest_by_market
            .insert(level.market_digest(), observation);
        Ok(observation)
    }

    pub fn check_price(
        &self,
        base_asset: AssetId,
        quote_asset: AssetId,
        price_numerator: u128,
        price_denominator: u128,
        accepted_deviation_bps: Bps,
        current_epoch: u64,
    ) -> AxisResult<PriceCheck> {
        let market_digest = Digest::from_parts(
            "axis-oracle-market-v1",
            &[&base_asset.bytes(), &quote_asset.bytes()],
        );
        let observation = self
            .latest_by_market
            .get(&market_digest)
            .ok_or_else(|| AxisError::Policy("oracle market not published".to_owned()))?;
        let age = current_epoch.saturating_sub(observation.level.updated_at_epoch);
        if age > self.stale_after_epochs {
            return Err(AxisError::Policy("oracle observation is stale".to_owned()));
        }
        let observed_deviation_bps = observation
            .level
            .deviation_bps(price_numerator, price_denominator)?;
        if observed_deviation_bps > accepted_deviation_bps {
            return Err(AxisError::Policy("quote outside oracle band".to_owned()));
        }
        Ok(PriceCheck {
            market_digest,
            observed_deviation_bps,
            accepted_deviation_bps,
            confidence_bps: observation.level.confidence_bps,
        })
    }

    pub fn latest(&self, base_asset: AssetId, quote_asset: AssetId) -> Option<OracleObservation> {
        let market_digest = Digest::from_parts(
            "axis-oracle-market-v1",
            &[&base_asset.bytes(), &quote_asset.bytes()],
        );
        self.latest_by_market.get(&market_digest).copied()
    }
}

impl Default for OracleRegistry {
    fn default() -> Self {
        Self::new(64)
    }
}
