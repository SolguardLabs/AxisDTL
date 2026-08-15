export const BPS = 10_000n;

export function asBigInt(value, field = "amount") {
    if (typeof value === "bigint") {
        if (value < 0n) throw new RangeError(`${field} must be non-negative`);
        return value;
    }
    if (typeof value === "number") {
        if (!Number.isSafeInteger(value) || value < 0) {
            throw new RangeError(`${field} must be a non-negative safe integer`);
        }
        return BigInt(value);
    }
    if (typeof value !== "string" || !/^(0|[1-9][0-9]*)$/.test(value)) {
        throw new TypeError(`${field} must be an unsigned integer string`);
    }
    return BigInt(value);
}

export function applyBps(value, bps) {
    if (!Number.isInteger(bps) || bps < 0 || bps > 10_000) {
        throw new RangeError("bps must be an integer between 0 and 10000");
    }
    return (asBigInt(value) * BigInt(bps)) / BPS;
}

export function reserveRequirement(netPayable, bufferBps) {
    const payable = asBigInt(netPayable, "netPayable");
    return payable + applyBps(payable, bufferBps);
}

export function compressionBps(gross, payable) {
    const grossAmount = asBigInt(gross, "gross");
    const payableAmount = asBigInt(payable, "payable");
    if (payableAmount > grossAmount) throw new RangeError("payable exceeds gross obligations");
    if (grossAmount === 0n) return 0;
    return Number(((grossAmount - payableAmount) * BPS) / grossAmount);
}

export function previewNetting(obligations, reserves = {}, bufferBps = 0) {
    const positions = new Map();
    const assets = new Map();
    const references = new Set();
    for (const obligation of obligations) {
        const amount = asBigInt(obligation.amount);
        if (amount === 0n) throw new RangeError("obligation amount must be positive");
        if (obligation.debtor === obligation.creditor)
            throw new RangeError("obligation parties must differ");
        if (references.has(obligation.reference))
            throw new RangeError("obligation reference is duplicated");
        references.add(obligation.reference);
        const asset = assets.get(obligation.asset) ?? { asset: obligation.asset, gross: 0n };
        asset.gross += amount;
        assets.set(obligation.asset, asset);
        addPosition(positions, obligation.debtor, obligation.asset, amount, 0n);
        addPosition(positions, obligation.creditor, obligation.asset, 0n, amount);
    }

    const rows = [...positions.values()]
        .map((position) => ({
            ...position,
            netDebit: position.debit > position.credit ? position.debit - position.credit : 0n,
            netCredit: position.credit > position.debit ? position.credit - position.debit : 0n,
        }))
        .sort((left, right) =>
            `${left.asset}:${left.account}`.localeCompare(`${right.asset}:${right.account}`),
        );
    const summaries = [...assets.values()]
        .map((asset) => {
            const payable = rows
                .filter((position) => position.asset === asset.asset)
                .reduce((sum, position) => sum + position.netDebit, 0n);
            const availableReserve = asBigInt(reserves[asset.asset] ?? 0n, "availableReserve");
            const requiredReserve = reserveRequirement(payable, bufferBps);
            return {
                asset: asset.asset,
                gross: asset.gross,
                payable,
                compressed: asset.gross - payable,
                compressionBps: compressionBps(asset.gross, payable),
                requiredReserve,
                availableReserve,
                shortfall:
                    requiredReserve > availableReserve ? requiredReserve - availableReserve : 0n,
            };
        })
        .sort((left, right) => left.asset.localeCompare(right.asset));
    return {
        obligationCount: obligations.length,
        positionCount: rows.length,
        fullyReserved: summaries.every((summary) => summary.shortfall === 0n),
        positions: rows,
        assets: summaries,
    };
}

function addPosition(positions, account, asset, debit, credit) {
    const key = `${asset}\u0000${account}`;
    const current = positions.get(key) ?? { account, asset, debit: 0n, credit: 0n };
    current.debit += debit;
    current.credit += credit;
    positions.set(key, current);
}

export function planRouteCapacity(used, limit, requested, reserveFloor = 0n) {
    const usedAmount = asBigInt(used, "used");
    const limitAmount = asBigInt(limit, "limit");
    const requestedAmount = asBigInt(requested, "requested");
    const floor = asBigInt(reserveFloor, "reserveFloor");
    const effectiveLimit = limitAmount > floor ? limitAmount - floor : 0n;
    const remaining = effectiveLimit > usedAmount ? effectiveLimit - usedAmount : 0n;
    const accepted = requestedAmount <= remaining;
    const projected = usedAmount + requestedAmount;
    const utilizationBps =
        effectiveLimit === 0n
            ? 10_000
            : Number(
                  (projected * BPS > effectiveLimit * BPS
                      ? effectiveLimit * BPS
                      : projected * BPS) / effectiveLimit,
              );
    return {
        accepted,
        requested: requestedAmount,
        effectiveLimit,
        remaining: accepted ? remaining - requestedAmount : remaining,
        utilizationBps,
        reason: accepted ? "accepted" : "capacity-exceeded",
    };
}

export function assertDigest(value, field = "digest") {
    if (typeof value !== "string" || !/^[0-9a-f]{64}$/i.test(value)) {
        throw new TypeError(`${field} must be a 32-byte hexadecimal digest`);
    }
    return value.toLowerCase();
}

export class AxisClient {
    constructor(executor) {
        if (typeof executor !== "function") throw new TypeError("executor must be a function");
        this.executor = executor;
    }

    async scenario(name = "routed") {
        const output = await this.executor([name]);
        const report = JSON.parse(output);
        assertDigest(report.state_digest, "state_digest");
        return report;
    }

    async health() {
        const report = await this.scenario("snapshot");
        return {
            networkId: report.network_id,
            conserved: report.conservation_ok === true,
            journalEntries: report.journal_entries,
            surface: report.surface,
            stateDigest: report.state_digest,
        };
    }
}
