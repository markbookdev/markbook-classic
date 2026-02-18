#!/usr/bin/env node
/* eslint-disable no-console */
const cp = require("node:child_process");
const fs = require("node:fs");
const path = require("node:path");

function run(cmd, args, opts = {}) {
  const res = cp.spawnSync(cmd, args, { stdio: "inherit", ...opts });
  if (res.error) throw res.error;
  if (res.status !== 0) {
    throw new Error(`${cmd} ${args.join(" ")} failed with exit code ${res.status}`);
  }
}

function main() {
  if (process.env.MARKBOOK_SKIP_SIDECAR_STAGE) {
    console.log("stage-sidecar: skipping (MARKBOOK_SKIP_SIDECAR_STAGE=1)");
    return;
  }

  // scripts/ is under apps/desktop; repo root is 3 levels up.
  const repoRoot = path.join(__dirname, "..", "..", "..");
  const rustDir = path.join(repoRoot, "rust", "markbookd");

  const binName = process.platform === "win32" ? "markbookd.exe" : "markbookd";
  const src = path.join(rustDir, "target", "release", binName);
  const destDir = path.join(repoRoot, "apps", "desktop", "resources", "markbookd");
  const dest = path.join(destDir, binName);

  console.log("stage-sidecar: building (cargo build --release)...");
  run("cargo", ["build", "--release"], { cwd: rustDir });

  if (!fs.existsSync(src)) {
    throw new Error(`stage-sidecar: expected binary missing: ${src}`);
  }

  fs.mkdirSync(destDir, { recursive: true });
  fs.copyFileSync(src, dest);
  if (process.platform !== "win32") {
    try {
      fs.chmodSync(dest, 0o755);
    } catch {
      // ignore
    }
  }

  const st = fs.statSync(dest);
  console.log(`stage-sidecar: staged ${dest} (${st.size} bytes)`);
}

main();

