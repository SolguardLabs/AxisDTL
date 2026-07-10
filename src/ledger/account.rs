use std::collections::BTreeMap;

use serde::Serialize;

use crate::{AccountId, Amount, AssetId, AxisError, AxisResult, PublicIdentity};

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct AccountState {
    pub identity: PublicIdentity,
    pub balances: BTreeMap<AssetId, Amount>,
    pub next_swap_nonce: u64,
    pub next_settlement_nonce: u64,
}

impl AccountState {
    pub fn new(identity: PublicIdentity) -> Self {
        Self {
            identity,
            balances: BTreeMap::new(),
            next_swap_nonce: 0,
            next_settlement_nonce: 0,
        }
    }

    pub fn balance_of(&self, asset: AssetId) -> Amount {
        self.balances
            .get(&asset)
            .copied()
            .unwrap_or_else(Amount::zero)
    }

    pub(crate) fn credit(&mut self, asset: AssetId, amount: Amount) -> AxisResult<()> {
        let next = self.balance_of(asset).checked_add(amount)?;
        self.set_balance(asset, next);
        Ok(())
    }

    pub(crate) fn debit(
        &mut self,
        account: AccountId,
        asset: AssetId,
        amount: Amount,
    ) -> AxisResult<()> {
        let available = self.balance_of(asset);
        if available < amount {
            return Err(AxisError::InsufficientFunds {
                account,
                asset,
                available,
                required: amount,
            });
        }
        self.set_balance(asset, available.checked_sub(amount)?);
        Ok(())
    }

    pub(crate) fn advance_swap_nonce(&mut self) -> AxisResult<()> {
        self.next_swap_nonce = self
            .next_swap_nonce
            .checked_add(1)
            .ok_or(AxisError::NonceOverflow)?;
        Ok(())
    }

    pub(crate) fn advance_settlement_nonce(&mut self) -> AxisResult<()> {
        self.next_settlement_nonce = self
            .next_settlement_nonce
            .checked_add(1)
            .ok_or(AxisError::NonceOverflow)?;
        Ok(())
    }

    fn set_balance(&mut self, asset: AssetId, amount: Amount) {
        if amount.is_zero() {
            self.balances.remove(&asset);
        } else {
            self.balances.insert(asset, amount);
        }
    }
}
