use std::collections::{BTreeMap, BTreeSet};

use serde::Serialize;

use crate::{
    AccountId, AccountRiskProfile, AccountState, Amount, AssetConfig, AssetId, AxisError,
    AxisResult, CustodyState, Digest, JournalEntry, JournalOp, MarginAccount, OracleRegistry,
    OrderId, PriceLevel, ProtocolLimits, PublicIdentity, RiskEngine, RouteBook, RoutePlan,
    SignedSettlement, SignedSwap, TreasuryPolicy, TxId, VaultConfig, VenueConfig,
};

#[derive(Clone, Debug, Serialize)]
pub struct AxisLedger {
    network_id: u32,
    pool: AccountId,
    assets: BTreeMap<AssetId, AssetConfig>,
    accounts: BTreeMap<AccountId, AccountState>,
    settled_orders: BTreeSet<OrderId>,
    seen_transactions: BTreeSet<TxId>,
    total_supply: BTreeMap<AssetId, Amount>,
    journal: Vec<JournalEntry>,
    route_book: RouteBook,
    oracle: OracleRegistry,
    risk_engine: RiskEngine,
    custody: CustodyState,
}

#[derive(Serialize)]
struct LedgerDigestView<'a> {
    network_id: u32,
    pool: AccountId,
    assets: &'a BTreeMap<AssetId, AssetConfig>,
    accounts: &'a BTreeMap<AccountId, AccountState>,
    settled_orders: &'a BTreeSet<OrderId>,
    seen_transactions: &'a BTreeSet<TxId>,
    total_supply: &'a BTreeMap<AssetId, Amount>,
    route_book: &'a RouteBook,
    oracle: &'a OracleRegistry,
    risk_engine: &'a RiskEngine,
    custody: &'a CustodyState,
    journal_len: usize,
}

impl AxisLedger {
    pub fn new(network_id: u32, pool: AccountId) -> Self {
        Self {
            network_id,
            pool,
            assets: BTreeMap::new(),
            accounts: BTreeMap::new(),
            settled_orders: BTreeSet::new(),
            seen_transactions: BTreeSet::new(),
            total_supply: BTreeMap::new(),
            journal: Vec::new(),
            route_book: RouteBook::default(),
            oracle: OracleRegistry::default(),
            risk_engine: RiskEngine::default(),
            custody: CustodyState::default(),
        }
    }

    pub const fn network_id(&self) -> u32 {
        self.network_id
    }

    pub const fn pool(&self) -> AccountId {
        self.pool
    }

    pub fn register_asset(&mut self, config: AssetConfig) -> AxisResult<()> {
        if self.assets.contains_key(&config.id) {
            return Err(AxisError::AssetAlreadyExists(config.id));
        }
        self.assets.insert(config.id, config);
        self.total_supply
            .entry(config.id)
            .or_insert_with(Amount::zero);
        Ok(())
    }

    pub fn register_account(&mut self, identity: PublicIdentity) -> AxisResult<()> {
        identity.verify_consistency()?;
        if self.accounts.contains_key(&identity.account) {
            return Err(AxisError::AccountAlreadyExists(identity.account));
        }
        self.accounts
            .insert(identity.account, AccountState::new(identity));
        Ok(())
    }

    pub fn credit_genesis(
        &mut self,
        account: AccountId,
        asset: AssetId,
        amount: Amount,
    ) -> AxisResult<TxId> {
        if amount.is_zero() {
            return Err(AxisError::ZeroAmount);
        }
        self.asset_config(asset)?;
        self.credit(account, asset, amount)?;
        let supply = self.total_supply_of(asset).checked_add(amount)?;
        self.total_supply.insert(asset, supply);
        self.verify_conservation(asset)?;

        let tx_id = TxId::from_serializable(
            "axis-genesis-credit-v1",
            &(
                self.network_id,
                account,
                asset,
                amount,
                self.journal.len() as u64,
            ),
        )?;
        self.append_journal(
            tx_id,
            JournalOp::GenesisCredit {
                account,
                asset,
                amount,
            },
        )?;
        Ok(tx_id)
    }

    pub fn balance_of(&self, account: AccountId, asset: AssetId) -> AxisResult<Amount> {
        Ok(self.account(account)?.balance_of(asset))
    }

    pub fn swap_nonce(&self, account: AccountId) -> AxisResult<u64> {
        Ok(self.account(account)?.next_swap_nonce)
    }

    pub fn settlement_nonce(&self, account: AccountId) -> AxisResult<u64> {
        Ok(self.account(account)?.next_settlement_nonce)
    }

    pub fn total_supply_of(&self, asset: AssetId) -> Amount {
        self.total_supply
            .get(&asset)
            .copied()
            .unwrap_or_else(Amount::zero)
    }

    pub fn asset_config(&self, asset: AssetId) -> AxisResult<AssetConfig> {
        self.assets
            .get(&asset)
            .copied()
            .ok_or(AxisError::AssetNotFound(asset))
    }

    pub fn journal(&self) -> &[JournalEntry] {
        &self.journal
    }

    pub fn route_count(&self) -> usize {
        self.route_book.route_count()
    }

    pub fn venue_count(&self) -> usize {
        self.route_book.venue_count()
    }

    pub fn vault_count(&self) -> usize {
        self.custody.vault_count()
    }

    pub fn margin_count(&self) -> usize {
        self.custody.margin_count()
    }

    pub fn set_protocol_limits(&mut self, limits: ProtocolLimits) {
        self.risk_engine.set_limits(limits);
    }

    pub fn register_oracle_publisher(
        &mut self,
        publisher: AccountId,
        weight: u16,
    ) -> AxisResult<()> {
        self.account(publisher)?;
        self.oracle.register_publisher(publisher, weight)
    }

    pub fn publish_price(&mut self, publisher: AccountId, level: PriceLevel) -> AxisResult<TxId> {
        self.account(publisher)?;
        let observation = self.oracle.publish(publisher, level)?;
        let tx_id = TxId::from_serializable(
            "axis-oracle-publish-tx-v1",
            &(self.network_id, observation, self.journal.len() as u64),
        )?;
        self.append_journal(
            tx_id,
            JournalOp::OraclePublished {
                feed_id: observation.feed_id,
                publisher,
                market_digest: level.market_digest(),
            },
        )?;
        Ok(tx_id)
    }

    pub fn register_venue(&mut self, config: VenueConfig) -> AxisResult<TxId> {
        self.account(config.operator)?;
        self.route_book.register_venue(config)?;
        let tx_id = TxId::from_serializable(
            "axis-venue-register-tx-v1",
            &(self.network_id, config, self.journal.len() as u64),
        )?;
        self.append_journal(
            tx_id,
            JournalOp::VenueRegistered {
                venue_id: config.venue_id,
                operator: config.operator,
            },
        )?;
        Ok(tx_id)
    }

    pub fn register_route(&mut self, plan: RoutePlan) -> AxisResult<TxId> {
        self.route_book.register_route(plan.clone())?;
        let tx_id = TxId::from_serializable(
            "axis-route-register-tx-v1",
            &(self.network_id, &plan, self.journal.len() as u64),
        )?;
        self.append_journal(
            tx_id,
            JournalOp::RouteRegistered {
                route_id: plan.route_id,
                quote_digest: plan.quote_digest,
                leg_count: plan.legs.len() as u8,
            },
        )?;
        Ok(tx_id)
    }

    pub fn set_risk_profile(&mut self, profile: AccountRiskProfile) -> AxisResult<TxId> {
        self.account(profile.account)?;
        self.risk_engine.set_profile(profile);
        let tx_id = TxId::from_serializable(
            "axis-risk-profile-tx-v1",
            &(self.network_id, profile, self.journal.len() as u64),
        )?;
        self.append_journal(
            tx_id,
            JournalOp::RiskProfileUpdated {
                account: profile.account,
                enabled: profile.enabled,
            },
        )?;
        Ok(tx_id)
    }

    pub fn register_vault(&mut self, config: VaultConfig) -> AxisResult<TxId> {
        self.account(config.account)?;
        self.account(config.custodian)?;
        self.custody.register_vault(config)?;
        let tx_id = TxId::from_serializable(
            "axis-vault-register-tx-v1",
            &(self.network_id, config, self.journal.len() as u64),
        )?;
        self.append_journal(
            tx_id,
            JournalOp::VaultRegistered {
                account: config.account,
                custodian: config.custodian,
                reserve_asset: config.reserve_asset,
                reserve_floor: config.reserve_floor,
            },
        )?;
        Ok(tx_id)
    }

    pub fn configure_treasury(&mut self, policy: TreasuryPolicy) -> AxisResult<TxId> {
        self.account(policy.fee_recipient)?;
        self.custody.set_treasury(policy.clone());
        let tx_id = TxId::from_serializable(
            "axis-treasury-configure-tx-v1",
            &(self.network_id, &policy, self.journal.len() as u64),
        )?;
        self.append_journal(
            tx_id,
            JournalOp::TreasuryConfigured {
                fee_recipient: policy.fee_recipient,
                protocol_fee_bps: policy.protocol_fee_bps,
            },
        )?;
        Ok(tx_id)
    }

    pub fn open_margin(&mut self, account: MarginAccount) -> AxisResult<TxId> {
        self.account(account.owner)?;
        self.asset_config(account.collateral_asset)?;
        self.custody.open_margin(account)?;
        let tx_id = TxId::from_serializable(
            "axis-margin-open-tx-v1",
            &(self.network_id, account, self.journal.len() as u64),
        )?;
        self.append_journal(
            tx_id,
            JournalOp::MarginOpened {
                margin_id: account.margin_id,
                owner: account.owner,
                collateral_asset: account.collateral_asset,
                collateral: account.collateral,
            },
        )?;
        Ok(tx_id)
    }

    pub fn state_digest(&self) -> AxisResult<Digest> {
        Digest::from_serializable(
            "axis-ledger-state-v1",
            &LedgerDigestView {
                network_id: self.network_id,
                pool: self.pool,
                assets: &self.assets,
                accounts: &self.accounts,
                settled_orders: &self.settled_orders,
                seen_transactions: &self.seen_transactions,
                total_supply: &self.total_supply,
                route_book: &self.route_book,
                oracle: &self.oracle,
                risk_engine: &self.risk_engine,
                custody: &self.custody,
                journal_len: self.journal.len(),
            },
        )
    }

    pub fn is_conserved(&self, asset: AssetId) -> AxisResult<bool> {
        self.verify_conservation(asset)?;
        Ok(true)
    }

    pub fn execute_swap(
        &mut self,
        signed_swap: &SignedSwap,
        signed_settlement: &SignedSettlement,
    ) -> AxisResult<TxId> {
        let mut candidate = self.clone();
        let tx_id = candidate.apply_swap(signed_swap, signed_settlement)?;
        *self = candidate;
        Ok(tx_id)
    }

    fn apply_swap(
        &mut self,
        signed_swap: &SignedSwap,
        signed_settlement: &SignedSettlement,
    ) -> AxisResult<TxId> {
        signed_swap.verify()?;
        signed_settlement.verify()?;

        let terms = signed_swap.terms;
        let request = signed_settlement.request;
        let quote = terms.quote;

        if terms.network_id != self.network_id || request.network_id != self.network_id {
            return Err(AxisError::Policy("network mismatch".to_owned()));
        }
        if request.order_id != terms.order_id {
            return Err(AxisError::Policy("settlement order mismatch".to_owned()));
        }
        if request.solver != quote.solver {
            return Err(AxisError::UnauthorizedSettlementSigner {
                expected: quote.solver,
                received: request.solver,
            });
        }
        if self.settled_orders.contains(&terms.order_id) {
            return Err(AxisError::OrderSettled(terms.order_id));
        }

        let tx_id =
            TxId::from_serializable("axis-swap-execution-v1", &(signed_swap, signed_settlement))?;
        if self.seen_transactions.contains(&tx_id) {
            return Err(AxisError::DuplicateTransaction(tx_id));
        }

        let expected_quote_digest = quote.digest()?;
        if request.observed_quote_digest != expected_quote_digest {
            return Err(AxisError::QuoteDigestMismatch {
                order_id: terms.order_id,
                expected: expected_quote_digest,
                received: request.observed_quote_digest,
            });
        }
        let limits = self.risk_engine.limits();
        if quote.expires_at_epoch < limits.current_epoch {
            return Err(AxisError::Policy("quote expired".to_owned()));
        }
        let route_quality = self.route_book.quality_for_quote(
            expected_quote_digest,
            terms.source_asset,
            terms.target_asset,
            terms.source_amount,
            limits.current_epoch,
        )?;
        let price_check = self.oracle.check_price(
            terms.source_asset,
            terms.target_asset,
            quote.price_numerator,
            quote.price_denominator,
            limits.oracle_deviation_bps,
            limits.current_epoch,
        )?;

        if self.swap_nonce(terms.payer)? != terms.payer_nonce {
            return Err(AxisError::NonceMismatch {
                account: terms.payer,
                expected: self.swap_nonce(terms.payer)?,
                received: terms.payer_nonce,
            });
        }
        if self.settlement_nonce(request.solver)? != request.settlement_nonce {
            return Err(AxisError::NonceMismatch {
                account: request.solver,
                expected: self.settlement_nonce(request.solver)?,
                received: request.settlement_nonce,
            });
        }

        self.account(terms.receiver)?;
        self.account(request.solver)?;
        self.account(self.pool)?;

        let source = self.asset_config(terms.source_asset)?;
        let target = self.asset_config(terms.target_asset)?;
        let gross_output = quote.output_amount(terms.source_amount, source, target)?;
        if gross_output < terms.min_output {
            return Err(AxisError::Policy("minimum output not reached".to_owned()));
        }
        let risk_decision = self
            .risk_engine
            .evaluate(terms, gross_output, route_quality)?;
        let solver_fee = quote.fee_amount(gross_output)?;
        let receiver_amount = gross_output.checked_sub(solver_fee)?;
        let pool_target_balance = self.balance_of(self.pool, terms.target_asset)?;
        self.custody.validate_reserve_after_debit(
            self.pool,
            terms.target_asset,
            pool_target_balance,
            gross_output,
        )?;

        self.debit(terms.payer, terms.source_asset, terms.source_amount)?;
        self.credit(self.pool, terms.source_asset, terms.source_amount)?;
        self.debit(self.pool, terms.target_asset, gross_output)?;
        self.credit(terms.receiver, terms.target_asset, receiver_amount)?;
        if !solver_fee.is_zero() {
            self.credit(request.solver, terms.target_asset, solver_fee)?;
        }

        self.account_mut(terms.payer)?.advance_swap_nonce()?;
        self.account_mut(request.solver)?
            .advance_settlement_nonce()?;
        self.risk_engine.record_execution(risk_decision)?;
        self.custody
            .record_solver_fee(terms.target_asset, solver_fee)?;
        self.settled_orders.insert(terms.order_id);
        self.seen_transactions.insert(tx_id);

        self.verify_conservation(terms.source_asset)?;
        self.verify_conservation(terms.target_asset)?;
        self.append_journal(
            tx_id,
            JournalOp::SwapExecution {
                order_id: terms.order_id,
                payer: terms.payer,
                receiver: terms.receiver,
                solver: request.solver,
                pool: self.pool,
                source_asset: terms.source_asset,
                target_asset: terms.target_asset,
                source_amount: terms.source_amount,
                gross_output,
                solver_fee,
                receiver_amount,
                route_quality: Box::new(route_quality),
                price_check: Box::new(price_check),
                risk_decision: Box::new(risk_decision),
            },
        )?;
        Ok(tx_id)
    }

    fn append_journal(&mut self, tx_id: TxId, op: JournalOp) -> AxisResult<()> {
        let entry = JournalEntry {
            sequence: self.journal.len() as u64,
            tx_id,
            op,
            state_digest: self.state_digest()?,
        };
        self.journal.push(entry);
        Ok(())
    }

    fn account(&self, account: AccountId) -> AxisResult<&AccountState> {
        self.accounts
            .get(&account)
            .ok_or(AxisError::AccountNotFound(account))
    }

    fn account_mut(&mut self, account: AccountId) -> AxisResult<&mut AccountState> {
        self.accounts
            .get_mut(&account)
            .ok_or(AxisError::AccountNotFound(account))
    }

    fn credit(&mut self, account: AccountId, asset: AssetId, amount: Amount) -> AxisResult<()> {
        self.account_mut(account)?.credit(asset, amount)
    }

    fn debit(&mut self, account: AccountId, asset: AssetId, amount: Amount) -> AxisResult<()> {
        self.account_mut(account)?.debit(account, asset, amount)
    }

    fn verify_conservation(&self, asset: AssetId) -> AxisResult<()> {
        let observed = self
            .accounts
            .values()
            .try_fold(Amount::zero(), |accumulator, account| {
                accumulator.checked_add(account.balance_of(asset))
            })?;
        let expected = self.total_supply_of(asset);
        if observed != expected {
            return Err(AxisError::Conservation {
                asset,
                expected,
                observed,
            });
        }
        Ok(())
    }
}
