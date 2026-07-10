use serde::Serialize;

use crate::{AssetId, AxisError, AxisResult};

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize)]
pub struct AssetConfig {
    pub id: AssetId,
    pub symbol: &'static str,
    pub decimals: u8,
}

impl AssetConfig {
    pub fn new(symbol: &'static str, decimals: u8) -> AxisResult<Self> {
        if decimals > 18 {
            return Err(AxisError::InvalidDecimals(decimals));
        }
        Ok(Self {
            id: AssetId::derive(symbol, decimals),
            symbol,
            decimals,
        })
    }
}
