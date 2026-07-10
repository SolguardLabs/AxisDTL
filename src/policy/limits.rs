use serde::Serialize;

use crate::{Amount, AxisError, AxisResult, Bps};

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ProtocolLimits {
    pub current_epoch: u64,
    pub max_order_amount: Amount,
    pub max_output_amount: Amount,
    pub max_solver_fee_bps: Bps,
    pub max_route_hops: u8,
    pub oracle_deviation_bps: Bps,
}

impl ProtocolLimits {
    pub fn new(
        current_epoch: u64,
        max_order_amount: Amount,
        max_output_amount: Amount,
        max_solver_fee_bps: Bps,
        max_route_hops: u8,
        oracle_deviation_bps: Bps,
    ) -> AxisResult<Self> {
        if max_order_amount.is_zero() || max_output_amount.is_zero() {
            return Err(AxisError::Policy(
                "protocol amount limit is zero".to_owned(),
            ));
        }
        if max_route_hops == 0 {
            return Err(AxisError::Policy(
                "protocol route hop limit is zero".to_owned(),
            ));
        }
        Ok(Self {
            current_epoch,
            max_order_amount,
            max_output_amount,
            max_solver_fee_bps,
            max_route_hops,
            oracle_deviation_bps,
        })
    }
}

impl Default for ProtocolLimits {
    fn default() -> Self {
        Self {
            current_epoch: 1_700,
            max_order_amount: Amount::new(100_000_000_000).expect("non-zero amount"),
            max_output_amount: Amount::new(100_000_000_000).expect("non-zero amount"),
            max_solver_fee_bps: Bps::new(100).expect("valid bps"),
            max_route_hops: 4,
            oracle_deviation_bps: Bps::new(250).expect("valid bps"),
        }
    }
}
