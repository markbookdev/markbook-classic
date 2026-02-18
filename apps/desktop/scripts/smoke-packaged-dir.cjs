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

function existsNonEmpty(p) {
  try {
    const st = fs.statSync(p);
    return st.isFile() && st.size > 1024;
  } catch {
    return false;
  }
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

function main() {
  const appDir = path.join(__dirname, "..");
  const outDir = path.join(appDir, "out");
  const binName = process.platform === "win32" ? "markbookd.exe" : "markbookd";

  console.log("smoke-packaged-dir: building (renderer + electron --dir)...");
  run("bun", ["run", "--cwd", appDir, "build"]);

  let expected = null;
  if (process.platform === "darwin") {
    const macDirs = fs
      .readdirSync(outDir, { withFileTypes: true })
      .filter((d) => d.isDirectory() && d.name.startsWith("mac"))
      .map((d) => path.join(outDir, d.name));
    if (macDirs.length === 0) {
      throw new Error(`smoke-packaged-dir: no mac* output dir found under ${outDir}`);
    }

    let appBundle = null;
    for (const dir of macDirs) {
      appBundle = findFirstAppBundle(dir);
      if (appBundle) break;
    }
    if (!appBundle) {
      throw new Error(
        `smoke-packaged-dir: no .app bundle found under ${macDirs.join(", ")}`
      );
    }
    expected = path.join(appBundle, "Contents", "Resources", "markbookd", binName);
  } else if (process.platform === "win32") {
    expected = path.join(outDir, "win-unpacked", "resources", "markbookd", binName);
  } else {
    expected = path.join(outDir, "linux-unpacked", "resources", "markbookd", binName);
  }

  if (!existsNonEmpty(expected)) {
    throw new Error(`smoke-packaged-dir: expected sidecar missing or empty: ${expected}`);
  }

  console.log(`smoke-packaged-dir: OK (${expected})`);
}

main();
