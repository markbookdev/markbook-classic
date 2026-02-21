#!/usr/bin/env node
/* eslint-disable no-console */
const fs = require("node:fs");
const path = require("node:path");
const crypto = require("node:crypto");

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

function sha256File(absPath) {
  const hash = crypto.createHash("sha256");
  hash.update(fs.readFileSync(absPath));
  return hash.digest("hex");
}

function collectChecksums(manifest) {
  const raw = manifest?.checksums;
  if (!raw || typeof raw !== "object") return {};
  const out = {};
  for (const [rel, expected] of Object.entries(raw)) {
    if (typeof rel !== "string" || typeof expected !== "string") continue;
    const key = rel.trim();
    const val = expected.trim().toLowerCase();
    if (!key || !val) continue;
    out[key] = val;
  }
  return out;
}

function checksumMismatches(baseDir, relPaths, checksumMap) {
  const out = [];
  for (const rel of relPaths) {
    const expected = checksumMap[rel];
    if (!expected) continue;
    const abs = path.join(baseDir, rel);
    if (!fs.existsSync(abs)) continue;
    const actual = sha256File(abs).toLowerCase();
    if (actual !== expected) {
      out.push({
        path: rel,
        expectedSha256: expected,
        actualSha256: actual
      });
    }
  }
  return out;
}

function printHumanStatus(payload, expectedDir, manifestPath) {
  console.log("parity-status: Sample25 lanes");
  console.log(`- manifest: ${manifestPath}`);
  console.log(`- status: ${payload.statusCode}`);
  console.log(`- regression required: ${payload.regression.requiredCount}`);
  console.log(`- strict required: ${payload.strict.requiredCount}`);
  console.log(`- strict configured: ${payload.strict.requiredByManifest ? "yes" : "no"}`);
  console.log(`- strict truth status: ${payload.strictTruth.status}`);

  if (payload.regression.missing.length > 0) {
    console.error("parity-status: regression lane missing required files:");
    for (const rel of payload.regression.missing) {
      console.error(`  - ${path.join(expectedDir, rel)}`);
    }
  } else if (payload.regression.checksumMismatches.length > 0) {
    console.error("parity-status: regression lane checksum mismatches:");
    for (const mismatch of payload.regression.checksumMismatches) {
      console.error(
        `  - ${path.join(expectedDir, mismatch.path)} expected=${mismatch.expectedSha256} actual=${mismatch.actualSha256}`
      );
    }
  } else {
    console.log("- regression lane: READY");
  }

  if (payload.strict.missing.length > 0 || payload.strict.checksumMismatches.length > 0) {
    const tag = payload.strict.requiredByManifest ? "NOT READY" : "PENDING (missing files)";
    console.log(`- strict lane: ${tag}`);
    for (const rel of payload.strict.missing) {
      console.log(`  - ${path.join(expectedDir, rel)}`);
    }
    for (const mismatch of payload.strict.checksumMismatches) {
      console.log(
        `  - ${path.join(expectedDir, mismatch.path)} expected=${mismatch.expectedSha256} actual=${mismatch.actualSha256}`
      );
    }
  } else {
    console.log("- strict lane: READY");
  }
}

function main() {
  const jsonMode = process.argv.includes("--json");
  const expectedManifestVersion = 1;
  const appDir = path.join(__dirname, "..");
  const repoRoot = path.join(appDir, "..", "..");
  const expectedDir = path.join(repoRoot, "fixtures", "legacy", "Sample25", "expected");
  const manifestPath = path.join(expectedDir, "parity-manifest.json");

  if (!fs.existsSync(manifestPath)) {
    throw new Error(`parity-status: missing manifest ${manifestPath}`);
  }

  const manifest = readJson(manifestPath);
  const manifestVersion = Number(manifest?.version || 0);
  const staleSchemaVersion = manifestVersion !== expectedManifestVersion;
  const strictRequiredByManifest = manifest?.strictReady === true;
  const regressionRequired = requiredList(manifest, "regression");
  const strictRequired = requiredList(manifest, "strict");
  const checksums = collectChecksums(manifest);
  const regressionMissing = missingFiles(expectedDir, regressionRequired);
  const strictMissing = missingFiles(expectedDir, strictRequired);
  const regressionChecksumMismatches = checksumMismatches(expectedDir, regressionRequired, checksums);
  const strictChecksumMismatches = checksumMismatches(expectedDir, strictRequired, checksums);
  const regressionReady = regressionMissing.length === 0 && regressionChecksumMismatches.length === 0;
  const strictFilesReady = strictMissing.length === 0 && strictChecksumMismatches.length === 0;
  const strictReady = !strictRequiredByManifest || strictFilesReady;
  const overallReady = !staleSchemaVersion && regressionReady && strictReady;
  let statusCode = "ready";
  if (staleSchemaVersion) {
    statusCode = "schema-mismatch";
  } else if (
    regressionChecksumMismatches.length > 0 ||
    (strictRequiredByManifest && strictChecksumMismatches.length > 0)
  ) {
    statusCode = "checksum-mismatch";
  } else if (!overallReady) {
    statusCode = "not-ready";
  }

  // Strict-truth lane status is intentionally independent from strictReady/requiredByManifest.
  // This gives an operator-readable "can we flip now?" signal at any time.
  let strictTruthStatus = "ready";
  if (staleSchemaVersion) {
    strictTruthStatus = "schema-mismatch";
  } else if (strictChecksumMismatches.length > 0) {
    strictTruthStatus = "checksum-mismatch";
  } else if (strictMissing.length > 0) {
    strictTruthStatus = "not-ready";
  }

  const payload = {
    mode: overallReady ? "ready" : "not-ready",
    statusCode,
    manifestPath,
    manifest: {
      version: manifestVersion,
      expectedVersion: expectedManifestVersion,
      staleSchemaVersion
    },
    artifacts: {
      expectedDir,
      regressionRequiredAbs: regressionRequired.map((rel) => path.join(expectedDir, rel)),
      strictRequiredAbs: strictRequired.map((rel) => path.join(expectedDir, rel))
    },
    checksums: {
      configuredCount: Object.keys(checksums).length
    },
    regression: {
      requiredCount: regressionRequired.length,
      ready: regressionReady,
      missing: regressionMissing,
      checksumMismatches: regressionChecksumMismatches
    },
    strict: {
      requiredByManifest: strictRequiredByManifest,
      requiredCount: strictRequired.length,
      filesReady: strictFilesReady,
      ready: strictReady,
      missing: strictMissing,
      checksumMismatches: strictChecksumMismatches
    },
    strictTruth: {
      status: strictTruthStatus,
      missing: strictMissing,
      checksumMismatches: strictChecksumMismatches
    }
  };

  if (!jsonMode) {
    printHumanStatus(payload, expectedDir, manifestPath);
    if (staleSchemaVersion) {
      console.error(
        `parity-status: stale manifest schema version ${manifestVersion}; expected ${expectedManifestVersion}`
      );
    }
  }
  console.log(JSON.stringify(payload));

  if (!overallReady) process.exitCode = 1;
}

main();
