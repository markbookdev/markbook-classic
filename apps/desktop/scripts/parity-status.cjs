#!/usr/bin/env node
/* eslint-disable no-console */
const fs = require("node:fs");
const path = require("node:path");

function readJson(filePath) {
  return JSON.parse(fs.readFileSync(filePath, "utf8"));
}

function requiredList(manifest, lane) {
  const arr = manifest?.required?.[lane];
  if (!Array.isArray(arr)) return [];
  return arr.filter((v) => typeof v === "string" && v.trim() !== "");
}

function missingFiles(baseDir, relPaths) {
  return relPaths.filter((rel) => !fs.existsSync(path.join(baseDir, rel)));
}

function printHumanStatus(payload, expectedDir, manifestPath) {
  console.log("parity-status: Sample25 lanes");
  console.log(`- manifest: ${manifestPath}`);
  console.log(`- regression required: ${payload.regression.requiredCount}`);
  console.log(`- strict required: ${payload.strict.requiredCount}`);
  console.log(`- strict configured: ${payload.strict.requiredByManifest ? "yes" : "no"}`);

  if (payload.regression.missing.length > 0) {
    console.error("parity-status: regression lane missing required files:");
    for (const rel of payload.regression.missing) {
      console.error(`  - ${path.join(expectedDir, rel)}`);
    }
  } else {
    console.log("- regression lane: READY");
  }

  if (payload.strict.missing.length > 0) {
    const tag = payload.strict.requiredByManifest ? "NOT READY" : "PENDING (missing files)";
    console.log(`- strict lane: ${tag}`);
    for (const rel of payload.strict.missing) {
      console.log(`  - ${path.join(expectedDir, rel)}`);
    }
  } else {
    console.log("- strict lane: READY");
  }
}

function main() {
  const jsonMode = process.argv.includes("--json");
  const appDir = path.join(__dirname, "..");
  const repoRoot = path.join(appDir, "..", "..");
  const expectedDir = path.join(repoRoot, "fixtures", "legacy", "Sample25", "expected");
  const manifestPath = path.join(expectedDir, "parity-manifest.json");

  if (!fs.existsSync(manifestPath)) {
    throw new Error(`parity-status: missing manifest ${manifestPath}`);
  }

  const manifest = readJson(manifestPath);
  const strictRequiredByManifest = manifest?.strictReady === true;
  const regressionRequired = requiredList(manifest, "regression");
  const strictRequired = requiredList(manifest, "strict");
  const regressionMissing = missingFiles(expectedDir, regressionRequired);
  const strictMissing = missingFiles(expectedDir, strictRequired);
  const regressionReady = regressionMissing.length === 0;
  const strictFilesReady = strictMissing.length === 0;
  const strictReady = !strictRequiredByManifest || strictFilesReady;
  const overallReady = regressionReady && strictReady;

  const payload = {
    mode: overallReady ? "ready" : "not-ready",
    manifestPath,
    regression: {
      requiredCount: regressionRequired.length,
      ready: regressionReady,
      missing: regressionMissing
    },
    strict: {
      requiredByManifest: strictRequiredByManifest,
      requiredCount: strictRequired.length,
      filesReady: strictFilesReady,
      ready: strictReady,
      missing: strictMissing
    }
  };

  if (!jsonMode) {
    printHumanStatus(payload, expectedDir, manifestPath);
  }
  console.log(JSON.stringify(payload));

  if (!overallReady) process.exitCode = 1;
}

main();
