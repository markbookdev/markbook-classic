#!/usr/bin/env node
/* eslint-disable no-console */
const cp = require("node:child_process");
const path = require("node:path");

function runParityStatus(repoRoot) {
  const script = path.join(repoRoot, "apps", "desktop", "scripts", "parity-status.cjs");
  const out = cp.execFileSync(process.execPath, [script, "--json"], {
    cwd: repoRoot,
    encoding: "utf8"
  });
  const lines = out
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter(Boolean);
  const jsonLine = lines[lines.length - 1];
  return JSON.parse(jsonLine);
}

function runTruthSuites(repoRoot) {
  const env = {
    ...process.env,
    MBC_STRICT_FRESH_SUMMARIES: "1"
  };
  const args = [
    "test",
    "--manifest-path",
    "rust/markbookd/Cargo.toml",
    "--test",
    "final_marks_vs_fresh_legacy_exports",
    "--test",
    "assessment_stats_vs_fresh_legacy_summaries"
  ];
  const result = cp.spawnSync("cargo", args, {
    cwd: repoRoot,
    env,
    stdio: "inherit"
  });
  return result.status ?? 1;
}

function main() {
  const repoRoot = path.join(__dirname, "..", "..", "..");
  const statusPayload = runParityStatus(repoRoot);
  const strictRequiredByManifest = Boolean(
    statusPayload?.strict?.requiredByManifest
  );
  const strictTruthStatus = statusPayload?.strictTruth?.status || "not-ready";

  const base = {
    mode: strictTruthStatus,
    strictRequiredByManifest,
    manifest: statusPayload?.manifest || {},
    missing: statusPayload?.strictTruth?.missing || [],
    checksumMismatches: statusPayload?.strictTruth?.checksumMismatches || []
  };

  if (strictTruthStatus === "ready") {
    const code = runTruthSuites(repoRoot);
    const payload = { ...base, suitesRan: true, suitesExitCode: code };
    console.log(JSON.stringify(payload));
    process.exit(code);
  }

  // Keep this runner non-blocking while strict is not required, but still fail fast for
  // manifest/schema/checksum problems regardless of strictReady.
  let exitCode = 0;
  if (
    strictTruthStatus === "schema-mismatch" ||
    strictTruthStatus === "checksum-mismatch"
  ) {
    exitCode = 1;
  } else if (strictRequiredByManifest) {
    exitCode = 1;
  }
  const payload = {
    ...base,
    suitesRan: false,
    suitesExitCode: null,
    skippedReason:
      strictTruthStatus === "not-ready"
        ? "strict artifacts missing"
        : "strict truth preflight failed"
  };
  console.log(JSON.stringify(payload));
  process.exit(exitCode);
}

main();

