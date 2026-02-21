const test = require("node:test");
const assert = require("node:assert/strict");
const cp = require("node:child_process");
const path = require("node:path");

const repoRoot = path.join(__dirname, "..", "..", "..", "..");
const scriptPath = path.join(repoRoot, "apps", "desktop", "scripts", "parity-status.cjs");

function runParityStatus(args = []) {
  const result = cp.spawnSync(process.execPath, [scriptPath, ...args], {
    cwd: repoRoot,
    encoding: "utf8"
  });
  return {
    status: result.status ?? 1,
    stdout: result.stdout || "",
    stderr: result.stderr || ""
  };
}

function parseLastJsonLine(stdout) {
  const lines = stdout
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter(Boolean);
  return JSON.parse(lines[lines.length - 1]);
}

test("parity-status json includes strict artifact intake checklist", () => {
  const result = runParityStatus(["--json"]);
  assert.ok(result.stdout.length > 0, "expected parity-status stdout");
  const payload = parseLastJsonLine(result.stdout);

  assert.equal(typeof payload.statusCode, "string");
  assert.ok(payload.intakeChecklist, "missing intakeChecklist");
  assert.ok(Array.isArray(payload.intakeChecklist.strictArtifactPathsAbs));
  assert.equal(typeof payload.intakeChecklist.checksumCommand, "string");
  assert.ok(
    payload.intakeChecklist.checksumCommand.includes("shasum -a 256"),
    "checksum command should include shasum invocation"
  );
});

test("parity-status human output includes intake checklist section", () => {
  const result = runParityStatus();
  assert.ok(result.stdout.includes("strict artifact intake checklist"));
  assert.ok(result.stdout.includes("Update parity-manifest.json checksums"));
});
