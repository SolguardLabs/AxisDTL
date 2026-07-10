use std::collections::BTreeMap;

use serde::Serialize;

use crate::{
    AccountId, Amount, AssetId, AxisError, AxisResult, Bps, MarginAccount, TreasuryPolicy,
};

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize)]
pub struct VaultConfig {
    pub account: AccountId,
    pub custodian: AccountId,
    pub reserve_asset: AssetId,
    pub reserve_floor: Amount,
    pub rebalance_threshold_bps: Bps,
    pub enabled: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct CustodyState {
    vaults: BTreeMap<AccountId, VaultConfig>,
    margins: BTreeMap<crate::Digest, MarginAccount>,
    treasury: Option<TreasuryPolicy>,
}

impl VaultConfig {
    pub fn new(
        account: AccountId,
        custodian: AccountId,
        reserve_asset: AssetId,
        reserve_floor: Amount,
        rebalance_threshold_bps: Bps,
    ) -> Self {
        Self {
            account,
            custodian,
            reserve_asset,
            reserve_floor,
            rebalance_threshold_bps,
            enabled: true,
        }
    }
}

impl CustodyState {
    pub fn new() -> Self {
        Self {
            vaults: BTreeMap::new(),
            margins: BTreeMap::new(),
            treasury: None,
        }
    }

    pub fn set_treasury(&mut self, policy: TreasuryPolicy) {
        self.treasury = Some(policy);
    }

    pub fn register_vault(&mut self, config: VaultConfig) -> AxisResult<()> {
        if self.vaults.contains_key(&config.account) {
            return Err(AxisError::Policy("vault already registered".to_owned()));
        }
        self.vaults.insert(config.account, config);
        Ok(())
    }

    pub fn open_margin(&mut self, account: MarginAccount) -> AxisResult<()> {
        if self.margins.contains_key(&account.margin_id) {
            return Err(AxisError::Policy("margin already opened".to_owned()));
        }
        self.margins.insert(account.margin_id, account);
        Ok(())
    }

    pub fn validate_reserve_after_debit(
        &self,
        vault: AccountId,
        asset: AssetId,
        available: Amount,
        debit: Amount,
    ) -> AxisResult<()> {
        let Some(config) = self.vaults.get(&vault) else {
            return Ok(());
        };
        if !config.enabled || config.reserve_asset != asset {
            return Ok(());
        }
        let remaining = available.checked_sub(debit)?;
        if remaining < config.reserve_floor {
            return Err(AxisError::Policy("vault reserve floor reached".to_owned()));
        }
        Ok(())
    }

    pub fn record_solver_fee(&mut self, asset: AssetId, amount: Amount) -> AxisResult<()> {
        if let Some(treasury) = &mut self.treasury {
            treasury.record_solver_fee(asset, amount)?;
        }
        Ok(())
    }

    pub fn vault_count(&self) -> usize {
        self.vaults.len()
    }

    pub fn margin_count(&self) -> usize {
        self.margins.len()
    }
}

impl Default for CustodyState {
    fn default() -> Self {
        Self::new()
    }
}
