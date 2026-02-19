#!/usr/bin/env node
/* eslint-disable no-console */
const cp = require("node:child_process");
const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");

function sleep(ms) {
  Atomics.wait(new Int32Array(new SharedArrayBuffer(4)), 0, 0, ms);
}

function findFirstAppBundle(macDir) {
  try {
    const entries = fs.readdirSync(macDir);
    const app = entries.find((e) => e.endsWith(".app"));
    return app ? path.join(macDir, app) : null;
  } catch {
    return null;
  }
}

function findPackagedExecutable(outDir) {
  if (process.platform === "darwin") {
    const macDirs = fs
      .readdirSync(outDir, { withFileTypes: true })
      .filter((d) => d.isDirectory() && d.name.startsWith("mac"))
      .map((d) => path.join(outDir, d.name));
    for (const macDir of macDirs) {
      const appBundle = findFirstAppBundle(macDir);
      if (!appBundle) continue;
      const macosDir = path.join(appBundle, "Contents", "MacOS");
      if (!fs.existsSync(macosDir)) continue;
      const bins = fs
        .readdirSync(macosDir)
        .filter((x) => !x.startsWith("."))
        .map((x) => path.join(macosDir, x))
        .filter((p) => {
          try {
            return fs.statSync(p).isFile();
          } catch {
            return false;
          }
        });
      if (bins.length > 0) return bins[0];
    }
    return null;
  }

  if (process.platform === "win32") {
    const winDir = path.join(outDir, "win-unpacked");
    if (!fs.existsSync(winDir)) return null;
    const exeCandidates = fs
      .readdirSync(winDir)
      .filter((name) => name.toLowerCase().endsWith(".exe"))
      .filter((name) => {
        const lower = name.toLowerCase();
        return !lower.includes("squirrel") && !lower.includes("uninstall");
      })
      .map((name) => path.join(winDir, name));
    return exeCandidates[0] || null;
  }

  const linuxDir = path.join(outDir, "linux-unpacked");
  if (!fs.existsSync(linuxDir)) return null;
  const entries = fs
    .readdirSync(linuxDir)
    .map((name) => path.join(linuxDir, name))
    .filter((p) => {
      try {
        return fs.statSync(p).isFile() && !p.endsWith(".so");
      } catch {
        return false;
      }
    });
  return entries[0] || null;
}

function readJson(filePath) {
  try {
    return JSON.parse(fs.readFileSync(filePath, "utf8"));
  } catch {
    return null;
  }
}

function waitForReadyFile(filePath, timeoutMs, child) {
  const startedAt = Date.now();
  let lastJson = null;

  while (Date.now() - startedAt < timeoutMs) {
    if (child.exitCode != null) {
      throw new Error(
        `packaged app exited early with code ${child.exitCode}; last ready payload: ${JSON.stringify(
          lastJson
        )}`
      );
    }

    const data = readJson(filePath);
    if (data) {
      lastJson = data;
      if (data.rendererLoaded && data.sidecarRunning && typeof data.sidecarPath === "string") {
        return data;
      }
    }
    sleep(200);
  }

  throw new Error(
    `timed out waiting for packaged ready file ${filePath}; last payload: ${JSON.stringify(
      lastJson
    )}`
  );
}

function main() {
  const appDir = path.join(__dirname, "..");
  const outDir = path.join(appDir, "out");
  const exePath = findPackagedExecutable(outDir);
  if (!exePath) {
    throw new Error(`smoke-packaged-launch: no packaged executable found under ${outDir}`);
  }

  const userDataDir = fs.mkdtempSync(path.join(os.tmpdir(), "markbook-packaged-launch-"));
  const readyFile =
    process.env.MARKBOOK_E2E_READY_FILE || path.join(userDataDir, "packaged-ready.json");
  fs.mkdirSync(path.dirname(readyFile), { recursive: true });
  try {
    fs.unlinkSync(readyFile);
  } catch {
    // ignore
  }

  console.log(`smoke-packaged-launch: launching ${exePath}`);
  console.log(`smoke-packaged-launch: ready file ${readyFile}`);
  const child = cp.spawn(exePath, [], {
    stdio: "ignore",
    env: {
      ...process.env,
      VITE_DEV_SERVER_URL: "",
      MARKBOOK_USER_DATA_DIR: userDataDir,
      MARKBOOK_E2E_READY_FILE: readyFile,
    },
  });

  try {
    const ready = waitForReadyFile(readyFile, 60_000, child);
    console.log(
      `smoke-packaged-launch: ready rendererLoaded=${Boolean(
        ready.rendererLoaded
      )} sidecarRunning=${Boolean(ready.sidecarRunning)} sidecarPath=${ready.sidecarPath}`
    );
    if (typeof ready.sidecarPath !== "string" || ready.sidecarPath.trim() === "") {
      throw new Error("smoke-packaged-launch: ready payload missing sidecarPath");
    }
  } finally {
    try {
      child.kill("SIGTERM");
    } catch {
      // ignore
    }
  }

  console.log("smoke-packaged-launch: OK");
}

main();
