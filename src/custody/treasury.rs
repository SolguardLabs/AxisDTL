use std::collections::BTreeMap;

use serde::Serialize;

use crate::{AccountId, Amount, AssetId, AxisResult, Bps};

#[derive(Clone, Debug, Serialize)]
pub struct TreasuryPolicy {
    pub fee_recipient: AccountId,
    pub protocol_fee_bps: Bps,
    pub accrued_fees: BTreeMap<AssetId, Amount>,
}

impl TreasuryPolicy {
    pub fn new(fee_recipient: AccountId, protocol_fee_bps: Bps) -> Self {
        Self {
            fee_recipient,
            protocol_fee_bps,
            accrued_fees: BTreeMap::new(),
        }
    }

    pub fn record_solver_fee(&mut self, asset: AssetId, amount: Amount) -> AxisResult<()> {
        let current = self
            .accrued_fees
            .get(&asset)
            .copied()
            .unwrap_or_else(Amount::zero);
        self.accrued_fees
            .insert(asset, current.checked_add(amount)?);
        Ok(())
    }
}
