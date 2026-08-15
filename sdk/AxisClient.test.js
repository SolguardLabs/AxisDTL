import assert from "node:assert/strict";
import test from "node:test";

import {
    AxisClient,
    applyBps,
    asBigInt,
    assertDigest,
    compressionBps,
    planRouteCapacity,
    previewNetting,
    reserveRequirement,
} from "./AxisClient.js";

test("integer parser rejects lossy or signed inputs", () => {
    assert.equal(asBigInt("123456"), 123456n);
    assert.throws(() => asBigInt(-1), /non-negative/);
    assert.throws(() => asBigInt("1.5"), /unsigned integer/);
});

test("basis points and reserve buffer use integer arithmetic", () => {
    assert.equal(applyBps(1_000_001n, 25), 2_500n);
    assert.equal(reserveRequirement(1_000_000n, 1_000), 1_100_000n);
});

test("compression compares gross obligations with net payable", () => {
    assert.equal(compressionBps(210n, 60n), 7_142);
    assert.equal(compressionBps(0n, 0n), 0);
    assert.throws(() => compressionBps(10n, 11n), /exceeds/);
});

test("multilateral preview nets a three-party cycle", () => {
    const report = previewNetting(
        [
            { debtor: "a", creditor: "b", asset: "AXUSD", amount: 100n, reference: "r1" },
            { debtor: "b", creditor: "c", asset: "AXUSD", amount: 70n, reference: "r2" },
            { debtor: "c", creditor: "a", asset: "AXUSD", amount: 40n, reference: "r3" },
        ],
        { AXUSD: 65n },
        1_000,
    );
    assert.equal(report.assets[0].gross, 210n);
    assert.equal(report.assets[0].payable, 60n);
    assert.equal(report.assets[0].requiredReserve, 66n);
    assert.equal(report.assets[0].shortfall, 1n);
    assert.equal(report.fullyReserved, false);
});

test("netting keeps asset domains separate", () => {
    const report = previewNetting(
        [
            { debtor: "a", creditor: "b", asset: "AXUSD", amount: 30n, reference: "r1" },
            { debtor: "b", creditor: "a", asset: "AXEUR", amount: 20n, reference: "r2" },
        ],
        { AXUSD: 30n, AXEUR: 20n },
    );
    assert.deepEqual(
        report.assets.map((asset) => [asset.asset, asset.payable]),
        [
            ["AXEUR", 20n],
            ["AXUSD", 30n],
        ],
    );
    assert.equal(report.fullyReserved, true);
});

test("capacity plan protects the reserve floor", () => {
    assert.deepEqual(planRouteCapacity(40n, 100n, 25n, 10n), {
        accepted: true,
        requested: 25n,
        effectiveLimit: 90n,
        remaining: 25n,
        utilizationBps: 7_222,
        reason: "accepted",
    });
    assert.equal(planRouteCapacity(80n, 100n, 20n, 10n).reason, "capacity-exceeded");
});

test("digest guard accepts exactly 32 hexadecimal bytes", () => {
    assert.equal(assertDigest("AA".repeat(32)), "aa".repeat(32));
    assert.throws(() => assertDigest("aa"), /32-byte/);
});

test("client parses scenarios and derives a health view", async () => {
    const calls = [];
    const client = new AxisClient(async (args) => {
        calls.push(args);
        return JSON.stringify({
            scenario: args[0],
            network_id: 42_170,
            conservation_ok: true,
            journal_entries: 9,
            surface: { venues: 1, routes: 0, vaults: 1, margins: 1 },
            state_digest: "ab".repeat(32),
        });
    });
    const health = await client.health();
    assert.equal(health.networkId, 42_170);
    assert.equal(health.conserved, true);
    assert.deepEqual(calls, [["snapshot"]]);
});
