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

function main() {
  const appDir = path.join(__dirname, "..");
  const repoRoot = path.join(appDir, "..", "..");
  const expectedDir = path.join(repoRoot, "fixtures", "legacy", "Sample25", "expected");
  const manifestPath = path.join(expectedDir, "parity-manifest.json");

  if (!fs.existsSync(manifestPath)) {
    throw new Error(`parity-status: missing manifest ${manifestPath}`);
  }

  const manifest = readJson(manifestPath);
  const regressionRequired = requiredList(manifest, "regression");
  const strictRequired = requiredList(manifest, "strict");
  const regressionMissing = missingFiles(expectedDir, regressionRequired);
  const strictMissing = missingFiles(expectedDir, strictRequired);

  console.log("parity-status: Sample25 lanes");
  console.log(`- manifest: ${manifestPath}`);
  console.log(`- regression required: ${regressionRequired.length}`);
  console.log(`- strict required: ${strictRequired.length}`);

  if (regressionMissing.length > 0) {
    console.error("parity-status: regression lane missing required files:");
    for (const rel of regressionMissing) {
      console.error(`  - ${path.join(expectedDir, rel)}`);
    }
    process.exitCode = 1;
  } else {
    console.log("- regression lane: READY");
  }

  if (strictMissing.length > 0) {
    console.log("- strict lane: PENDING (missing files)");
    for (const rel of strictMissing) {
      console.log(`  - ${path.join(expectedDir, rel)}`);
    }
  } else {
    console.log("- strict lane: READY");
  }
}

main();
