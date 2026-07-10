use std::collections::BTreeMap;

use serde::Serialize;

use crate::{Amount, AssetId, AxisError, AxisResult, Digest, RoutePlan, RouteQuality, VenueConfig};

#[derive(Clone, Debug, Default, Serialize)]
pub struct RouteBook {
    venues: BTreeMap<Digest, VenueConfig>,
    routes_by_quote: BTreeMap<Digest, RoutePlan>,
}

impl RouteBook {
    pub fn register_venue(&mut self, config: VenueConfig) -> AxisResult<()> {
        if self.venues.contains_key(&config.venue_id) {
            return Err(AxisError::Policy("venue already registered".to_owned()));
        }
        self.venues.insert(config.venue_id, config);
        Ok(())
    }

    pub fn register_route(&mut self, plan: RoutePlan) -> AxisResult<()> {
        for leg in &plan.legs {
            let venue = self
                .venues
                .get(&leg.venue_id)
                .ok_or_else(|| AxisError::Policy("route references unknown venue".to_owned()))?;
            if !venue.enabled {
                return Err(AxisError::Policy(
                    "route references disabled venue".to_owned(),
                ));
            }
            if plan.legs.len() > usize::from(venue.max_hops) {
                return Err(AxisError::Policy(
                    "route exceeds venue hop policy".to_owned(),
                ));
            }
        }
        self.routes_by_quote.insert(plan.quote_digest, plan);
        Ok(())
    }

    pub fn quality_for_quote(
        &self,
        quote_digest: Digest,
        source_asset: AssetId,
        target_asset: AssetId,
        source_amount: Amount,
        current_epoch: u64,
    ) -> AxisResult<RouteQuality> {
        let route = self
            .routes_by_quote
            .get(&quote_digest)
            .ok_or_else(|| AxisError::Policy("quote has no registered route".to_owned()))?;
        route.quality(source_asset, target_asset, source_amount, current_epoch)
    }

    pub fn route_count(&self) -> usize {
        self.routes_by_quote.len()
    }

    pub fn venue_count(&self) -> usize {
        self.venues.len()
    }
}
