import { spawnSync } from "node:child_process";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "../..");

export function runScenario(scenario) {
    const args = ["run", "--quiet"];
    if (scenario) {
        args.push("--", scenario);
    }

    const result = spawnSync("cargo", args, {
        cwd: root,
        encoding: "utf8",
        windowsHide: true,
    });

    if (result.status !== 0) {
        throw new Error(
            [
                `cargo run failed with status ${result.status}`,
                `stdout: ${result.stdout}`,
                `stderr: ${result.stderr}`,
            ].join("\n"),
        );
    }

    return JSON.parse(result.stdout);
}

export function expectHexDigest(value) {
    if (typeof value !== "string" || !/^[0-9a-f]{64}$/u.test(value)) {
        throw new Error(`invalid digest: ${value}`);
    }
}
