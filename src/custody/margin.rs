use serde::Serialize;

use crate::{AccountId, Amount, AssetId, AxisError, AxisResult, Bps, Digest};

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MarginMode {
    Isolated,
    Cross,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize)]
pub struct MarginAccount {
    pub margin_id: Digest,
    pub owner: AccountId,
    pub collateral_asset: AssetId,
    pub collateral: Amount,
    pub borrowed: Amount,
    pub maintenance_bps: Bps,
    pub mode: MarginMode,
}

impl MarginAccount {
    pub fn new(
        owner: AccountId,
        collateral_asset: AssetId,
        collateral: Amount,
        borrowed: Amount,
        maintenance_bps: Bps,
        mode: MarginMode,
    ) -> AxisResult<Self> {
        if collateral.is_zero() {
            return Err(AxisError::Policy("margin collateral is zero".to_owned()));
        }
        let margin_id = Digest::from_serializable(
            "axis-margin-account-v1",
            &(
                owner,
                collateral_asset,
                collateral,
                borrowed,
                maintenance_bps,
                mode,
            ),
        )?;
        Ok(Self {
            margin_id,
            owner,
            collateral_asset,
            collateral,
            borrowed,
            maintenance_bps,
            mode,
        })
    }
}
