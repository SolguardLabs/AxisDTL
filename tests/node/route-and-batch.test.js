import assert from "node:assert/strict";
import test from "node:test";

import { expectHexDigest, runScenario } from "../helpers/axis-cli.js";

test("batch acumula dos ordenes y mantiene IDs unicos", () => {
    const report = runScenario("batch");

    assert.equal(report.scenario, "batch");
    assert.equal(report.order_ids.length, 2);
    assert.notEqual(report.order_ids[0], report.order_ids[1]);
    assert.equal(report.transaction_ids.length, 2);
    assert.notEqual(report.transaction_ids[0], report.transaction_ids[1]);
    assert.equal(report.surface.routes, 2);
    assert.equal(report.journal_entries, 13);
    assert.equal(report.balances.payer_source, 23_250_000_000);
    assert.equal(report.balances.receiver_target, 1_737_772_500);
    assert.equal(report.conservation_ok, true);
    expectHexDigest(report.state_digest);
});

test("auction ejecuta una ruta compuesta sin romper conservacion", () => {
    const report = runScenario("auction");

    assert.equal(report.scenario, "auction");
    assert.equal(report.order_ids.length, 1);
    assert.equal(report.transaction_ids.length, 1);
    assert.equal(report.surface.routes, 1);
    assert.equal(report.journal_entries, 11);
    assert.equal(report.balances.receiver_target, 1_775_049_505);
    assert.equal(report.balances.solver_target, 7_128_712);
    assert.equal(report.conservation_ok, true);
    expectHexDigest(report.state_digest);
});

test("la invocacion sin argumentos usa el escenario routed", () => {
    const report = runScenario();

    assert.equal(report.scenario, "routed");
    assert.equal(report.surface.routes, 1);
    assert.equal(report.conservation_ok, true);
});
