import assert from "node:assert/strict";
import test from "node:test";

import { expectHexDigest, runScenario } from "../helpers/axis-cli.js";

test("routed expone balances conservados y superficie operacional", () => {
    const report = runScenario("routed");

    assert.equal(report.scenario, "routed");
    assert.equal(report.conservation_ok, true);
    assert.deepEqual(report.surface, {
        venues: 1,
        routes: 1,
        vaults: 1,
        margins: 1,
    });
    assert.equal(report.balances.payer_source, 23_000_000_000);
    assert.equal(report.balances.receiver_target, 1_993_000_000);
    assert.equal(report.balances.solver_target, 7_000_000);
    assert.equal(report.balances.pool_target, 48_000_000_000);
    expectHexDigest(report.state_digest);
});

test("direct mantiene una liquidacion simple con una ruta registrada", () => {
    const report = runScenario("direct");

    assert.equal(report.scenario, "direct");
    assert.equal(report.order_ids.length, 1);
    assert.equal(report.transaction_ids.length, 1);
    assert.equal(report.surface.routes, 1);
    assert.equal(report.balances.receiver_target, 1_247_500_000);
    assert.equal(report.balances.solver_target, 2_500_000);
    assert.equal(report.conservation_ok, true);
});

test("snapshot inicializa configuracion sin ordenes ejecutadas", () => {
    const report = runScenario("snapshot");

    assert.equal(report.scenario, "snapshot");
    assert.equal(report.order_ids.length, 0);
    assert.equal(report.transaction_ids.length, 0);
    assert.equal(report.surface.venues, 1);
    assert.equal(report.surface.routes, 0);
    assert.equal(report.surface.vaults, 1);
    assert.equal(report.surface.margins, 1);
    assert.equal(report.journal_entries, 9);
    assert.equal(report.conservation_ok, true);
});
