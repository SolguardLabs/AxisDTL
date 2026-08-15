use std::collections::{BTreeMap, BTreeSet};

use serde::Serialize;

use crate::{AccountId, Amount, AssetId, AxisError, AxisResult, Bps, Digest};

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ClearingObligation {
    pub debtor: AccountId,
    pub creditor: AccountId,
    pub asset: AssetId,
    pub amount: Amount,
    pub window: u64,
    pub reference: Digest,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ClearingLimits {
    pub max_obligations: usize,
    pub max_accounts_per_asset: usize,
    pub reserve_buffer_bps: Bps,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize)]
pub struct NetPosition {
    pub account: AccountId,
    pub asset: AssetId,
    pub gross_debit: Amount,
    pub gross_credit: Amount,
    pub net_debit: Amount,
    pub net_credit: Amount,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct AssetClearingSummary {
    pub asset: AssetId,
    pub gross_obligations: Amount,
    pub net_payable: Amount,
    pub compressed_amount: Amount,
    pub compression_bps: Bps,
    pub required_reserve: Amount,
    pub available_reserve: Amount,
    pub reserve_shortfall: Amount,
    pub account_count: usize,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ClearingCycle {
    pub window: u64,
    pub obligation_count: usize,
    pub position_count: usize,
    pub digest: Digest,
    pub positions: Vec<NetPosition>,
    pub assets: Vec<AssetClearingSummary>,
}

#[derive(Serialize)]
struct ClearingDigestView<'a> {
    version: &'static str,
    window: u64,
    obligation_count: usize,
    positions: &'a [NetPosition],
    assets: &'a [AssetClearingSummary],
}

pub struct ClearingEngine {
    limits: ClearingLimits,
}

impl ClearingObligation {
    pub fn new(
        debtor: AccountId,
        creditor: AccountId,
        asset: AssetId,
        amount: Amount,
        window: u64,
        reference: Digest,
    ) -> AxisResult<Self> {
        if debtor == creditor {
            return Err(AxisError::Policy(
                "clearing obligation has identical parties".to_owned(),
            ));
        }
        if amount.is_zero() {
            return Err(AxisError::ZeroAmount);
        }
        if window == 0 {
            return Err(AxisError::Policy(
                "clearing window must be positive".to_owned(),
            ));
        }
        Ok(Self {
            debtor,
            creditor,
            asset,
            amount,
            window,
            reference,
        })
    }
}

impl ClearingLimits {
    pub fn new(
        max_obligations: usize,
        max_accounts_per_asset: usize,
        reserve_buffer_bps: Bps,
    ) -> AxisResult<Self> {
        if max_obligations == 0 || max_accounts_per_asset < 2 {
            return Err(AxisError::Policy(
                "clearing limits must permit a bilateral cycle".to_owned(),
            ));
        }
        Ok(Self {
            max_obligations,
            max_accounts_per_asset,
            reserve_buffer_bps,
        })
    }
}

impl ClearingCycle {
    pub fn fully_reserved(&self) -> bool {
        self.assets
            .iter()
            .all(|summary| summary.reserve_shortfall.is_zero())
    }

    pub fn asset(&self, asset: AssetId) -> Option<&AssetClearingSummary> {
        self.assets.iter().find(|summary| summary.asset == asset)
    }

    pub fn position(&self, account: AccountId, asset: AssetId) -> Option<&NetPosition> {
        self.positions
            .iter()
            .find(|position| position.account == account && position.asset == asset)
    }
}

impl ClearingEngine {
    pub const fn new(limits: ClearingLimits) -> Self {
        Self { limits }
    }

    pub fn preview(
        &self,
        window: u64,
        obligations: &[ClearingObligation],
        reserves: &BTreeMap<AssetId, Amount>,
    ) -> AxisResult<ClearingCycle> {
        if window == 0 {
            return Err(AxisError::Policy(
                "clearing preview window must be positive".to_owned(),
            ));
        }
        if obligations.is_empty() || obligations.len() > self.limits.max_obligations {
            return Err(AxisError::Policy(
                "clearing obligation count outside configured limits".to_owned(),
            ));
        }

        let mut gross_debits = BTreeMap::<(AccountId, AssetId), Amount>::new();
        let mut gross_credits = BTreeMap::<(AccountId, AssetId), Amount>::new();
        let mut gross_by_asset = BTreeMap::<AssetId, Amount>::new();
        let mut accounts_by_asset = BTreeMap::<AssetId, BTreeSet<AccountId>>::new();
        let mut references = BTreeSet::new();

        for obligation in obligations {
            if obligation.window != window {
                return Err(AxisError::Policy(
                    "clearing obligation belongs to another window".to_owned(),
                ));
            }
            if !references.insert(obligation.reference) {
                return Err(AxisError::Policy(
                    "clearing obligation reference is duplicated".to_owned(),
                ));
            }
            add_amount(
                &mut gross_debits,
                (obligation.debtor, obligation.asset),
                obligation.amount,
                "clearing gross debit",
            )?;
            add_amount(
                &mut gross_credits,
                (obligation.creditor, obligation.asset),
                obligation.amount,
                "clearing gross credit",
            )?;
            add_amount(
                &mut gross_by_asset,
                obligation.asset,
                obligation.amount,
                "clearing gross asset",
            )?;
            accounts_by_asset
                .entry(obligation.asset)
                .or_default()
                .extend([obligation.debtor, obligation.creditor]);
        }

        for accounts in accounts_by_asset.values() {
            if accounts.len() > self.limits.max_accounts_per_asset {
                return Err(AxisError::Policy(
                    "clearing account count exceeds configured limit".to_owned(),
                ));
            }
        }

        let mut keys = BTreeSet::new();
        keys.extend(gross_debits.keys().copied());
        keys.extend(gross_credits.keys().copied());
        let mut positions = Vec::with_capacity(keys.len());
        for (account, asset) in keys {
            let debit = gross_debits
                .get(&(account, asset))
                .copied()
                .unwrap_or_else(Amount::zero);
            let credit = gross_credits
                .get(&(account, asset))
                .copied()
                .unwrap_or_else(Amount::zero);
            let (net_debit, net_credit) = if debit >= credit {
                (debit.checked_sub(credit)?, Amount::zero())
            } else {
                (Amount::zero(), credit.checked_sub(debit)?)
            };
            positions.push(NetPosition {
                account,
                asset,
                gross_debit: debit,
                gross_credit: credit,
                net_debit,
                net_credit,
            });
        }

        let mut assets = Vec::with_capacity(gross_by_asset.len());
        for (asset, gross_obligations) in gross_by_asset {
            let net_payable = positions
                .iter()
                .filter(|position| position.asset == asset)
                .try_fold(Amount::zero(), |total, position| {
                    total.checked_add(position.net_debit)
                })?;
            let total_credit = positions
                .iter()
                .filter(|position| position.asset == asset)
                .try_fold(Amount::zero(), |total, position| {
                    total.checked_add(position.net_credit)
                })?;
            if net_payable != total_credit {
                return Err(AxisError::Policy(
                    "clearing net positions do not balance".to_owned(),
                ));
            }
            let compressed_amount = gross_obligations.checked_sub(net_payable)?;
            let compression_units = compressed_amount
                .units()
                .checked_mul(10_000)
                .ok_or(AxisError::AmountOverflow)?
                .checked_div(gross_obligations.units())
                .ok_or(AxisError::DivisionByZero)?;
            let compression_bps =
                Bps::new(u16::try_from(compression_units).map_err(|_| AxisError::AmountOverflow)?)?;
            let required_reserve = net_payable
                .checked_mul(10_000 + u128::from(self.limits.reserve_buffer_bps.units()))?
                .checked_div(10_000)?;
            let available_reserve = reserves.get(&asset).copied().unwrap_or_else(Amount::zero);
            let reserve_shortfall = if required_reserve > available_reserve {
                required_reserve.checked_sub(available_reserve)?
            } else {
                Amount::zero()
            };
            assets.push(AssetClearingSummary {
                asset,
                gross_obligations,
                net_payable,
                compressed_amount,
                compression_bps,
                required_reserve,
                available_reserve,
                reserve_shortfall,
                account_count: accounts_by_asset.get(&asset).map_or(0, BTreeSet::len),
            });
        }

        let digest = Digest::from_serializable(
            "axis-clearing-cycle-v1",
            &ClearingDigestView {
                version: "1",
                window,
                obligation_count: obligations.len(),
                positions: &positions,
                assets: &assets,
            },
        )?;
        Ok(ClearingCycle {
            window,
            obligation_count: obligations.len(),
            position_count: positions.len(),
            digest,
            positions,
            assets,
        })
    }
}

fn add_amount<Key: Ord + Copy>(
    totals: &mut BTreeMap<Key, Amount>,
    key: Key,
    amount: Amount,
    field: &str,
) -> AxisResult<()> {
    let current = totals.get(&key).copied().unwrap_or_else(Amount::zero);
    totals.insert(
        key,
        current
            .checked_add(amount)
            .map_err(|_| AxisError::Policy(format!("{field} overflow")))?,
    );
    Ok(())
}
