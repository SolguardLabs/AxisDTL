use serde::Serialize;

use crate::{
    AccountRiskProfile, Amount, AssetConfig, AssetId, AxisLedger, AxisResult, Bps, Digest,
    ExecutionQuote, KeyPair, MarginAccount, MarginMode, OrderId, PriceLevel, ProtocolLimits,
    RiskTier, RouteLeg, RoutePlan, SettlementRequest, SignedSettlement, SignedSwap, SwapTerms,
    TreasuryPolicy, TxId, VaultConfig, VenueConfig, VenueKind,
};

const NETWORK_ID: u32 = 42_170;

struct RuntimeFixture {
    ledger: AxisLedger,
    payer: KeyPair,
    receiver: KeyPair,
    solver: KeyPair,
    pool: KeyPair,
    source: AssetConfig,
    target: AssetConfig,
    bridge: AssetConfig,
    venue: VenueConfig,
}

#[derive(Debug, Serialize)]
pub struct ScenarioReport {
    pub scenario: String,
    pub network_id: u32,
    pub source_asset: AssetReport,
    pub target_asset: AssetReport,
    pub order_ids: Vec<OrderId>,
    pub transaction_ids: Vec<TxId>,
    pub balances: BalanceReport,
    pub supply: SupplyReport,
    pub surface: SurfaceReport,
    pub journal_entries: usize,
    pub state_digest: Digest,
    pub conservation_ok: bool,
}

#[derive(Debug, Serialize)]
pub struct AssetReport {
    pub id: AssetId,
    pub symbol: &'static str,
    pub decimals: u8,
}

#[derive(Debug, Serialize)]
pub struct BalanceReport {
    pub payer_source: Amount,
    pub receiver_target: Amount,
    pub solver_target: Amount,
    pub pool_source: Amount,
    pub pool_target: Amount,
}

#[derive(Debug, Serialize)]
pub struct SupplyReport {
    pub source: Amount,
    pub target: Amount,
}

#[derive(Debug, Serialize)]
pub struct SurfaceReport {
    pub venues: usize,
    pub routes: usize,
    pub vaults: usize,
    pub margins: usize,
}

pub fn run_named(name: &str) -> AxisResult<ScenarioReport> {
    match name {
        "direct" => run_direct(),
        "batch" => run_batch(),
        "auction" => run_auction(),
        "snapshot" => run_snapshot(),
        "routed" => run_routed(),
        _ => run_routed(),
    }
}

fn run_direct() -> AxisResult<ScenarioReport> {
    let mut fixture = fixture()?;
    let (order_id, tx_id) = execute_fixture_swap(
        &mut fixture,
        SwapInput {
            source_amount: 1_250_000_000,
            min_output: 1_240_000_000,
            price_numerator: 1,
            price_denominator: 1,
            fee_bps: 20,
            salt_label: "direct-primary",
            venue_label: "direct-rfq",
            route_shape: RouteShape::Direct,
        },
    )?;
    report(fixture, "direct", vec![order_id], vec![tx_id])
}

fn run_routed() -> AxisResult<ScenarioReport> {
    let mut fixture = fixture()?;
    let (order_id, tx_id) = execute_fixture_swap(
        &mut fixture,
        SwapInput {
            source_amount: 2_000_000_000,
            min_output: 1_970_000_000,
            price_numerator: 1,
            price_denominator: 1,
            fee_bps: 35,
            salt_label: "routed-primary",
            venue_label: "hybrid-route-a",
            route_shape: RouteShape::Direct,
        },
    )?;
    report(fixture, "routed", vec![order_id], vec![tx_id])
}

fn run_auction() -> AxisResult<ScenarioReport> {
    let mut fixture = fixture()?;
    let (order_id, tx_id) = execute_fixture_swap(
        &mut fixture,
        SwapInput {
            source_amount: 1_800_000_000,
            min_output: 1_760_000_000,
            price_numerator: 100,
            price_denominator: 101,
            fee_bps: 40,
            salt_label: "auction-primary",
            venue_label: "auction-route",
            route_shape: RouteShape::TwoHop,
        },
    )?;
    report(fixture, "auction", vec![order_id], vec![tx_id])
}

fn run_batch() -> AxisResult<ScenarioReport> {
    let mut fixture = fixture()?;
    let mut orders = Vec::new();
    let mut txs = Vec::new();
    let first = execute_fixture_swap(
        &mut fixture,
        SwapInput {
            source_amount: 1_000_000_000,
            min_output: 990_000_000,
            price_numerator: 1,
            price_denominator: 1,
            fee_bps: 25,
            salt_label: "batch-a",
            venue_label: "batch-route-a",
            route_shape: RouteShape::Direct,
        },
    )?;
    orders.push(first.0);
    txs.push(first.1);
    let second = execute_fixture_swap(
        &mut fixture,
        SwapInput {
            source_amount: 750_000_000,
            min_output: 735_000_000,
            price_numerator: 99,
            price_denominator: 100,
            fee_bps: 30,
            salt_label: "batch-b",
            venue_label: "batch-route-b",
            route_shape: RouteShape::Direct,
        },
    )?;
    orders.push(second.0);
    txs.push(second.1);
    report(fixture, "batch", orders, txs)
}

fn run_snapshot() -> AxisResult<ScenarioReport> {
    report(fixture()?, "snapshot", Vec::new(), Vec::new())
}

fn fixture() -> AxisResult<RuntimeFixture> {
    let payer = keyed(11);
    let receiver = keyed(22);
    let solver = keyed(33);
    let pool = keyed(44);
    let source = AssetConfig::new("AXUSD", 6)?;
    let target = AssetConfig::new("AXEUR", 6)?;
    let bridge = AssetConfig::new("AXBND", 6)?;
    let venue = VenueConfig::new(
        "axis-hybrid-rfq",
        solver.public_identity().account,
        VenueKind::ExternalRfq,
        Bps::new(5)?,
        Bps::new(12)?,
        4,
    )?;

    let mut ledger = AxisLedger::new(NETWORK_ID, pool.public_identity().account);
    ledger.set_protocol_limits(ProtocolLimits::new(
        1_700,
        Amount::new(100_000_000_000)?,
        Amount::new(100_000_000_000)?,
        Bps::new(100)?,
        4,
        Bps::new(250)?,
    )?);
    ledger.register_asset(source)?;
    ledger.register_asset(target)?;
    ledger.register_asset(bridge)?;
    ledger.register_account(payer.public_identity())?;
    ledger.register_account(receiver.public_identity())?;
    ledger.register_account(solver.public_identity())?;
    ledger.register_account(pool.public_identity())?;

    ledger.credit_genesis(
        payer.public_identity().account,
        source.id,
        Amount::new(25_000_000_000)?,
    )?;
    ledger.credit_genesis(
        pool.public_identity().account,
        source.id,
        Amount::new(5_000_000_000)?,
    )?;
    ledger.credit_genesis(
        pool.public_identity().account,
        target.id,
        Amount::new(50_000_000_000)?,
    )?;
    ledger.set_risk_profile(AccountRiskProfile::new(
        payer.public_identity().account,
        RiskTier::Professional,
    ))?;
    ledger.register_oracle_publisher(solver.public_identity().account, 100)?;
    ledger.publish_price(
        solver.public_identity().account,
        PriceLevel::new(source.id, target.id, 1, 1, Bps::new(20)?, 1_700)?,
    )?;
    ledger.register_venue(venue)?;
    ledger.register_vault(VaultConfig::new(
        pool.public_identity().account,
        solver.public_identity().account,
        target.id,
        Amount::new(10_000_000_000)?,
        Bps::new(750)?,
    ))?;
    ledger.configure_treasury(TreasuryPolicy::new(
        pool.public_identity().account,
        Bps::new(5)?,
    ))?;
    ledger.open_margin(MarginAccount::new(
        payer.public_identity().account,
        source.id,
        Amount::new(1_500_000_000)?,
        Amount::new(250_000_000)?,
        Bps::new(1_250)?,
        MarginMode::Cross,
    )?)?;

    Ok(RuntimeFixture {
        ledger,
        payer,
        receiver,
        solver,
        pool,
        source,
        target,
        bridge,
        venue,
    })
}

enum RouteShape {
    Direct,
    TwoHop,
}

struct SwapInput {
    source_amount: u128,
    min_output: u128,
    price_numerator: u128,
    price_denominator: u128,
    fee_bps: u16,
    salt_label: &'static str,
    venue_label: &'static str,
    route_shape: RouteShape,
}

fn execute_fixture_swap(
    fixture: &mut RuntimeFixture,
    input: SwapInput,
) -> AxisResult<(OrderId, TxId)> {
    let payer = fixture.payer.public_identity().account;
    let receiver = fixture.receiver.public_identity().account;
    let solver = fixture.solver.public_identity().account;
    let payer_nonce = fixture.ledger.swap_nonce(payer)?;
    let settlement_nonce = fixture.ledger.settlement_nonce(solver)?;
    let salt = Digest::from_parts("axis-runtime-salt-v1", &[input.salt_label.as_bytes()]);
    let venue_digest = Digest::from_parts("axis-runtime-venue-v1", &[input.venue_label.as_bytes()]);
    let quote = ExecutionQuote::new(
        solver,
        fixture.source.id,
        fixture.target.id,
        input.price_numerator,
        input.price_denominator,
        Bps::new(input.fee_bps)?,
        settlement_nonce,
        9_999_999,
        venue_digest,
    )?;
    let terms = SwapTerms::new(
        fixture.ledger.network_id(),
        payer,
        receiver,
        fixture.source.id,
        fixture.target.id,
        Amount::new(input.source_amount)?,
        Amount::new(input.min_output)?,
        payer_nonce,
        quote,
        salt,
    )?;
    register_route_for_terms(fixture, terms, input.venue_label, input.route_shape)?;
    let signed_swap = SignedSwap::sign(terms, &fixture.payer)?;
    let settlement_request = SettlementRequest::new(
        fixture.ledger.network_id(),
        terms.order_id,
        solver,
        settlement_nonce,
        quote.digest()?,
    );
    let signed_settlement = SignedSettlement::sign(settlement_request, &fixture.solver)?;
    let tx_id = fixture
        .ledger
        .execute_swap(&signed_swap, &signed_settlement)?;
    Ok((terms.order_id, tx_id))
}

fn register_route_for_terms(
    fixture: &mut RuntimeFixture,
    terms: SwapTerms,
    venue_label: &'static str,
    route_shape: RouteShape,
) -> AxisResult<()> {
    let quote_digest = terms.quote.digest()?;
    let route_liquidity = Amount::new(25_000_000_000)?;
    let legs = match route_shape {
        RouteShape::Direct => vec![RouteLeg::new(
            fixture.venue.venue_id,
            fixture.source.id,
            fixture.target.id,
            Bps::new(10_000)?,
            route_liquidity,
            18,
        )?],
        RouteShape::TwoHop => vec![
            RouteLeg::new(
                fixture.venue.venue_id,
                fixture.source.id,
                fixture.bridge.id,
                Bps::new(5_000)?,
                route_liquidity,
                24,
            )?,
            RouteLeg::new(
                fixture.venue.venue_id,
                fixture.bridge.id,
                fixture.target.id,
                Bps::new(5_000)?,
                route_liquidity,
                31,
            )?,
        ],
    };
    let route = RoutePlan::new(quote_digest, legs, 9_999_999, route_priority(venue_label))?;
    fixture.ledger.register_route(route)?;
    Ok(())
}

fn route_priority(label: &str) -> u8 {
    if label.contains("auction") { 2 } else { 1 }
}

fn report(
    fixture: RuntimeFixture,
    scenario: &'static str,
    order_ids: Vec<OrderId>,
    transaction_ids: Vec<TxId>,
) -> AxisResult<ScenarioReport> {
    let payer = fixture.payer.public_identity().account;
    let receiver = fixture.receiver.public_identity().account;
    let solver = fixture.solver.public_identity().account;
    let pool = fixture.pool.public_identity().account;
    let source = fixture.source.id;
    let target = fixture.target.id;
    Ok(ScenarioReport {
        scenario: scenario.to_owned(),
        network_id: fixture.ledger.network_id(),
        source_asset: asset_report(fixture.source),
        target_asset: asset_report(fixture.target),
        order_ids,
        transaction_ids,
        balances: BalanceReport {
            payer_source: fixture.ledger.balance_of(payer, source)?,
            receiver_target: fixture.ledger.balance_of(receiver, target)?,
            solver_target: fixture.ledger.balance_of(solver, target)?,
            pool_source: fixture.ledger.balance_of(pool, source)?,
            pool_target: fixture.ledger.balance_of(pool, target)?,
        },
        supply: SupplyReport {
            source: fixture.ledger.total_supply_of(source),
            target: fixture.ledger.total_supply_of(target),
        },
        surface: SurfaceReport {
            venues: fixture.ledger.venue_count(),
            routes: fixture.ledger.route_count(),
            vaults: fixture.ledger.vault_count(),
            margins: fixture.ledger.margin_count(),
        },
        journal_entries: fixture.ledger.journal().len(),
        state_digest: fixture.ledger.state_digest()?,
        conservation_ok: fixture.ledger.is_conserved(source)?
            && fixture.ledger.is_conserved(target)?,
    })
}

fn asset_report(config: AssetConfig) -> AssetReport {
    AssetReport {
        id: config.id,
        symbol: config.symbol,
        decimals: config.decimals,
    }
}

fn keyed(byte: u8) -> KeyPair {
    KeyPair::from_seed([byte; 32])
}
