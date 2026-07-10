import { spawnSync } from "node:child_process";
import { readdirSync, statSync } from "node:fs";
import { extname, join, relative, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const root = resolve(fileURLToPath(new URL("..", import.meta.url)));
const nodeBin = process.env.NODE_BIN ?? "node";
const checkedDirs = ["tests", "scripts"];
const checkedExtensions = new Set([".js", ".mjs"]);

function collectFiles(dir) {
  const absoluteDir = join(root, dir);
  const files = [];

  for (const entry of readdirSync(absoluteDir)) {
    const path = join(absoluteDir, entry);
    const stat = statSync(path);

    if (stat.isDirectory()) {
      files.push(...collectFiles(relative(root, path)));
      continue;
    }

    if (stat.isFile() && checkedExtensions.has(extname(path))) {
      files.push(path);
    }
  }

  return files;
}

const files = checkedDirs.flatMap(collectFiles).sort();

for (const file of files) {
  const result = spawnSync(nodeBin, ["--check", file], {
    cwd: root,
    encoding: "utf8",
    stdio: "pipe",
  });

  if (result.status !== 0) {
    process.stderr.write(result.stderr);
    process.stderr.write(result.stdout);
    process.exit(result.status ?? 1);
  }
}

console.log(`Sintaxis JavaScript verificada en ${files.length} archivos.`);
