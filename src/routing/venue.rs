use serde::Serialize;

use crate::{AccountId, AxisError, AxisResult, Bps, Digest};

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum VenueKind {
    InternalPool,
    ExternalRfq,
    Auction,
    LiquidityVault,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize)]
pub struct VenueConfig {
    pub venue_id: Digest,
    pub operator: AccountId,
    pub kind: VenueKind,
    pub maker_fee_bps: Bps,
    pub taker_fee_bps: Bps,
    pub max_hops: u8,
    pub enabled: bool,
}

impl VenueConfig {
    pub fn new(
        label: &str,
        operator: AccountId,
        kind: VenueKind,
        maker_fee_bps: Bps,
        taker_fee_bps: Bps,
        max_hops: u8,
    ) -> AxisResult<Self> {
        if max_hops == 0 {
            return Err(AxisError::Policy("venue max hops is zero".to_owned()));
        }
        Ok(Self {
            venue_id: Digest::from_parts("axis-venue-v1", &[label.as_bytes(), &operator.bytes()]),
            operator,
            kind,
            maker_fee_bps,
            taker_fee_bps,
            max_hops,
            enabled: true,
        })
    }
}
