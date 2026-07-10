use serde::Serialize;

use crate::{Amount, AssetId, AxisError, AxisResult, Bps, Digest};

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize)]
pub struct RouteLeg {
    pub venue_id: Digest,
    pub source_asset: AssetId,
    pub target_asset: AssetId,
    pub weight_bps: Bps,
    pub liquidity_limit: Amount,
    pub expected_latency_ms: u32,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct RoutePlan {
    pub route_id: Digest,
    pub quote_digest: Digest,
    pub legs: Vec<RouteLeg>,
    pub ttl_epoch: u64,
    pub priority: u8,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize)]
pub struct RouteQuality {
    pub route_id: Digest,
    pub hop_count: u8,
    pub aggregate_weight_bps: Bps,
    pub max_latency_ms: u32,
    pub tightest_liquidity_limit: Amount,
}

impl RouteLeg {
    pub fn new(
        venue_id: Digest,
        source_asset: AssetId,
        target_asset: AssetId,
        weight_bps: Bps,
        liquidity_limit: Amount,
        expected_latency_ms: u32,
    ) -> AxisResult<Self> {
        if liquidity_limit.is_zero() {
            return Err(AxisError::Policy(
                "route liquidity limit is zero".to_owned(),
            ));
        }
        Ok(Self {
            venue_id,
            source_asset,
            target_asset,
            weight_bps,
            liquidity_limit,
            expected_latency_ms,
        })
    }
}

impl RoutePlan {
    pub fn new(
        quote_digest: Digest,
        legs: Vec<RouteLeg>,
        ttl_epoch: u64,
        priority: u8,
    ) -> AxisResult<Self> {
        if legs.is_empty() {
            return Err(AxisError::Policy("route has no legs".to_owned()));
        }
        let route_id = Digest::from_serializable(
            "axis-route-plan-v1",
            &(quote_digest, &legs, ttl_epoch, priority),
        )?;
        Ok(Self {
            route_id,
            quote_digest,
            legs,
            ttl_epoch,
            priority,
        })
    }

    pub fn quality(
        &self,
        source_asset: AssetId,
        target_asset: AssetId,
        source_amount: Amount,
        current_epoch: u64,
    ) -> AxisResult<RouteQuality> {
        if self.ttl_epoch < current_epoch {
            return Err(AxisError::Policy("route expired".to_owned()));
        }
        if self.legs.first().map(|leg| leg.source_asset) != Some(source_asset) {
            return Err(AxisError::Policy("route source mismatch".to_owned()));
        }
        if self.legs.last().map(|leg| leg.target_asset) != Some(target_asset) {
            return Err(AxisError::Policy("route target mismatch".to_owned()));
        }

        let mut previous_target = source_asset;
        let mut aggregate_weight = 0u16;
        let mut max_latency_ms = 0u32;
        let mut tightest_limit = self.legs[0].liquidity_limit;
        for leg in &self.legs {
            if leg.source_asset != previous_target {
                return Err(AxisError::Policy(
                    "route leg continuity mismatch".to_owned(),
                ));
            }
            if leg.liquidity_limit < source_amount {
                return Err(AxisError::Policy(
                    "route liquidity below source amount".to_owned(),
                ));
            }
            aggregate_weight = aggregate_weight
                .checked_add(leg.weight_bps.units())
                .ok_or(AxisError::AmountOverflow)?;
            max_latency_ms = max_latency_ms.max(leg.expected_latency_ms);
            tightest_limit = tightest_limit.min(leg.liquidity_limit);
            previous_target = leg.target_asset;
        }
        Ok(RouteQuality {
            route_id: self.route_id,
            hop_count: self.legs.len() as u8,
            aggregate_weight_bps: Bps::new(aggregate_weight)?,
            max_latency_ms,
            tightest_liquidity_limit: tightest_limit,
        })
    }
}
