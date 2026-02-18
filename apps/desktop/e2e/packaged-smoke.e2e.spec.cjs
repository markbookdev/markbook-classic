const { test, expect } = require("@playwright/test");
const cp = require("node:child_process");
const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");

function mkdtemp(prefix) {
  return fs.mkdtempSync(path.join(os.tmpdir(), prefix));
}

function findFirstAppBundle(macDir) {
  const entries = fs.readdirSync(macDir);
  const app = entries.find((e) => e.endsWith(".app"));
  return app ? path.join(macDir, app) : null;
}

function findFirstMacOutDir(outDir) {
  const entries = fs.readdirSync(outDir, { withFileTypes: true });
  const macDirs = entries
    .filter((d) => d.isDirectory() && d.name.startsWith("mac"))
    .map((d) => path.join(outDir, d.name));
  return macDirs.length > 0 ? macDirs[0] : null;
}

function waitForReadyFile(p, timeoutMs) {
  const start = Date.now();
  while (Date.now() - start < timeoutMs) {
    try {
      const txt = fs.readFileSync(p, "utf8");
      const j = JSON.parse(txt);
      if (j?.rendererLoaded && j?.sidecarRunning) return j;
    } catch {
      // ignore
    }
    Atomics.wait(new Int32Array(new SharedArrayBuffer(4)), 0, 0, 200);
  }
  throw new Error(`timed out waiting for ready file: ${p}`);
}

test("packaged (dir) app starts renderer + sidecar", async () => {
  if (process.env.MARKBOOK_RUN_PACKAGED_E2E !== "1") {
    test.skip(true, "set MARKBOOK_RUN_PACKAGED_E2E=1 to run packaged smoke");
  }
  if (process.platform !== "darwin") {
    test.skip(true, "packaged smoke is macOS-only for now");
  }

  const repoRoot = path.join(__dirname, "..", "..", "..");
  const appDir = path.join(repoRoot, "apps", "desktop");
  const outDir = path.join(appDir, "out");

  const macOutDir = findFirstMacOutDir(outDir);
  expect(macOutDir).not.toBeNull();

  const appBundle = findFirstAppBundle(macOutDir);
  expect(appBundle).not.toBeNull();

  const macosDir = path.join(appBundle, "Contents", "MacOS");
  const macosEntries = fs.readdirSync(macosDir).filter((x) => !x.startsWith("."));
  expect(macosEntries.length).toBeGreaterThan(0);
  const exe = path.join(macosDir, macosEntries[0]);
  expect(fs.existsSync(exe)).toBeTruthy();

  const userDataDir = mkdtemp("markbook-userdata-packaged-");
  const readyFile = path.join(userDataDir, "ready.json");

  const child = cp.spawn(exe, [], {
    stdio: "ignore",
    env: {
      ...process.env,
      VITE_DEV_SERVER_URL: "",
      MARKBOOK_USER_DATA_DIR: userDataDir,
      MARKBOOK_E2E_READY_FILE: readyFile,
    },
  });

  try {
    const ready = waitForReadyFile(readyFile, 45_000);
    expect(ready.sidecarRunning).toBeTruthy();
    expect(typeof ready.sidecarPath).toBe("string");
  } finally {
    try {
      child.kill("SIGTERM");
    } catch {
      // ignore
    }
  }
});
