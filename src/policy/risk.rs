use std::collections::BTreeMap;

use serde::Serialize;

use crate::{AccountId, Amount, AxisError, AxisResult, ProtocolLimits, RouteQuality, SwapTerms};

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RiskTier {
    Retail,
    Professional,
    Institutional,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize)]
pub struct AccountRiskProfile {
    pub account: AccountId,
    pub tier: RiskTier,
    pub daily_volume: Amount,
    pub open_exposure: Amount,
    pub enabled: bool,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize)]
pub struct RiskDecision {
    pub account: AccountId,
    pub tier: RiskTier,
    pub route_id: crate::Digest,
    pub route_hops: u8,
    pub notional: Amount,
    pub projected_volume: Amount,
    pub accepted: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct RiskEngine {
    limits: ProtocolLimits,
    profiles: BTreeMap<AccountId, AccountRiskProfile>,
}

impl AccountRiskProfile {
    pub fn new(account: AccountId, tier: RiskTier) -> Self {
        Self {
            account,
            tier,
            daily_volume: Amount::zero(),
            open_exposure: Amount::zero(),
            enabled: true,
        }
    }
}

impl RiskEngine {
    pub fn new(limits: ProtocolLimits) -> Self {
        Self {
            limits,
            profiles: BTreeMap::new(),
        }
    }

    pub const fn limits(&self) -> ProtocolLimits {
        self.limits
    }

    pub fn set_limits(&mut self, limits: ProtocolLimits) {
        self.limits = limits;
    }

    pub fn set_profile(&mut self, profile: AccountRiskProfile) {
        self.profiles.insert(profile.account, profile);
    }

    pub fn evaluate(
        &self,
        terms: SwapTerms,
        gross_output: Amount,
        route: RouteQuality,
    ) -> AxisResult<RiskDecision> {
        if terms.source_amount > self.limits.max_order_amount {
            return Err(AxisError::Policy(
                "order exceeds protocol source limit".to_owned(),
            ));
        }
        if gross_output > self.limits.max_output_amount {
            return Err(AxisError::Policy(
                "order exceeds protocol output limit".to_owned(),
            ));
        }
        if terms.quote.solver_fee_bps > self.limits.max_solver_fee_bps {
            return Err(AxisError::Policy("solver fee exceeds policy".to_owned()));
        }
        if route.hop_count > self.limits.max_route_hops {
            return Err(AxisError::Policy(
                "route exceeds protocol hop limit".to_owned(),
            ));
        }
        let profile = self
            .profiles
            .get(&terms.payer)
            .ok_or_else(|| AxisError::Policy("payer risk profile not found".to_owned()))?;
        if !profile.enabled {
            return Err(AxisError::Policy("payer risk profile disabled".to_owned()));
        }
        let projected_volume = profile.daily_volume.checked_add(terms.source_amount)?;
        Ok(RiskDecision {
            account: terms.payer,
            tier: profile.tier,
            route_id: route.route_id,
            route_hops: route.hop_count,
            notional: terms.source_amount,
            projected_volume,
            accepted: true,
        })
    }

    pub fn record_execution(&mut self, decision: RiskDecision) -> AxisResult<()> {
        let profile = self
            .profiles
            .get_mut(&decision.account)
            .ok_or_else(|| AxisError::Policy("risk profile not found".to_owned()))?;
        profile.daily_volume = decision.projected_volume;
        profile.open_exposure = profile.open_exposure.checked_add(decision.notional)?;
        Ok(())
    }
}

impl Default for RiskEngine {
    fn default() -> Self {
        Self::new(ProtocolLimits::default())
    }
}
