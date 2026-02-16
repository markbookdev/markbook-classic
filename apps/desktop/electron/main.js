const path = require("node:path");
const fs = require("node:fs");
const readline = require("node:readline");
const { spawn } = require("node:child_process");
const { app, BrowserWindow, dialog, ipcMain } = require("electron");

let mainWindow = null;

// E2E / harness support: isolate Electron userData (prefs, etc.) under a temp dir.
// Must be set before we read/write prefs.
if (process.env.MARKBOOK_USER_DATA_DIR) {
  try {
    app.setPath("userData", process.env.MARKBOOK_USER_DATA_DIR);
  } catch {
    // ignore
  }
}

// Lightweight preferences persisted under Electron userData.
// Keep this in main (privileged) so renderer never touches filesystem.
function prefsPath() {
  return path.join(app.getPath("userData"), "prefs.json");
}

function normalizePrefs(p) {
  const recent = Array.isArray(p?.recentWorkspaces)
    ? p.recentWorkspaces.filter((x) => typeof x === "string" && x.trim() !== "")
    : [];
  const last =
    typeof p?.lastWorkspace === "string" && p.lastWorkspace.trim() !== ""
      ? p.lastWorkspace
      : null;
  return { recentWorkspaces: recent, lastWorkspace: last };
}

function readPrefs() {
  try {
    const raw = fs.readFileSync(prefsPath(), "utf8");
    return normalizePrefs(JSON.parse(raw));
  } catch {
    return { recentWorkspaces: [], lastWorkspace: null };
  }
}

function writePrefs(p) {
  const out = normalizePrefs(p);
  fs.mkdirSync(path.dirname(prefsPath()), { recursive: true });
  fs.writeFileSync(prefsPath(), JSON.stringify(out, null, 2), "utf8");
  return out;
}

function addRecentWorkspace(p, wsPath) {
  if (!wsPath || typeof wsPath !== "string") return p;
  const cur = normalizePrefs(p);
  const next = [wsPath, ...cur.recentWorkspaces.filter((x) => x !== wsPath)].slice(0, 10);
  return { ...cur, recentWorkspaces: next };
}

// markbookd sidecar process + simple request/response map
let sidecar = null;
let sidecarPathUsed = null;
let nextReqId = 1;
const pending = new Map(); // id -> { resolve, reject }
let sidecarWatchInterval = null;

function startSidecarWatcher() {
  // Dev convenience: when rebuilding the Rust binary, restart the running sidecar.
  if (!process.env.VITE_DEV_SERVER_URL) return;
  if (!sidecarPathUsed) return;
  if (sidecarWatchInterval) return;

  let lastMtimeMs;
  try {
    lastMtimeMs = fs.statSync(sidecarPathUsed).mtimeMs;
  } catch {
    return;
  }

  sidecarWatchInterval = setInterval(() => {
    if (!sidecar || !sidecarPathUsed) return;
    try {
      const mtimeMs = fs.statSync(sidecarPathUsed).mtimeMs;
      if (mtimeMs !== lastMtimeMs) {
        lastMtimeMs = mtimeMs;
        console.warn("markbookd binary changed; restarting sidecar");
        stopSidecar();
      }
    } catch {
      // ignore
    }
  }, 500);

  // Don't keep the app alive just for this dev watcher.
  if (typeof sidecarWatchInterval.unref === "function") sidecarWatchInterval.unref();
}

function stopSidecarWatcher() {
  if (!sidecarWatchInterval) return;
  clearInterval(sidecarWatchInterval);
  sidecarWatchInterval = null;
}

function getSidecarCandidatePaths() {
  // 1) packaged: extraResources -> <resources>/markbookd/<platform binary>
  const resourcesDir = process.resourcesPath;
  const platformBin =
    process.platform === "win32" ? "markbookd.exe" : "markbookd";
  const packaged = path.join(resourcesDir, "markbookd", platformBin);

  // 2) repo dev: rust/markbookd/target/debug/markbookd (or .exe)
  // __dirname = <repo>/apps/desktop/electron
  const repoRoot = path.join(__dirname, "..", "..", "..");
  const dev = path.join(
    repoRoot,
    "rust",
    "markbookd",
    "target",
    "debug",
    platformBin
  );

  // 3) repo dev release
  const devRel = path.join(
    repoRoot,
    "rust",
    "markbookd",
    "target",
    "release",
    platformBin
  );

  return [packaged, dev, devRel];
}

function startSidecar() {
  if (sidecar) return;

  const candidates = getSidecarCandidatePaths();
  const sidecarPath = candidates.find((p) => fs.existsSync(p));

  if (!sidecarPath) {
    // Renderer can still load; requests will error until the binary exists.
    console.warn(
      "markbookd not found. Looked for:\n" + candidates.map((p) => "  " + p).join("\n")
    );
    return;
  }

  sidecarPathUsed = sidecarPath;
  sidecar = spawn(sidecarPath, [], { stdio: ["pipe", "pipe", "pipe"] });
  startSidecarWatcher();

  sidecar.on("exit", (code, sig) => {
    console.warn("markbookd exited", { code, sig });
    sidecar = null;
    sidecarPathUsed = null;
    stopSidecarWatcher();
    for (const [id, p] of pending.entries()) {
      p.reject(new Error("markbookd exited"));
      pending.delete(id);
    }
  });

  sidecar.stderr.on("data", (buf) => {
    console.warn("markbookd stderr:", String(buf));
  });

  const rl = readline.createInterface({ input: sidecar.stdout });
  rl.on("line", (line) => {
    let msg;
    try {
      msg = JSON.parse(line);
    } catch {
      console.warn("bad sidecar json:", line);
      return;
    }

    const p = pending.get(msg.id);
    if (!p) return;
    pending.delete(msg.id);

    if (msg.ok) p.resolve(msg.result);
    else p.reject(Object.assign(new Error(msg?.error?.message || "sidecar error"), { sidecarError: msg.error }));
  });
}

function stopSidecar() {
  if (!sidecar) return;

  for (const [id, p] of pending.entries()) {
    p.reject(new Error("markbookd restarted"));
    pending.delete(id);
  }

  try {
    sidecar.kill();
  } catch {
    // ignore
  }
  sidecar = null;
  sidecarPathUsed = null;
  stopSidecarWatcher();
}

function sidecarRequest(method, params) {
  startSidecar();
  if (!sidecar) {
    return Promise.reject(
      new Error("markbookd not running (build rust/markbookd first)")
    );
  }

  const id = String(nextReqId++);
  const payload = { id, method, params: params || {} };

  return new Promise((resolve, reject) => {
    pending.set(id, { resolve, reject });
    sidecar.stdin.write(JSON.stringify(payload) + "\n");
  });
}

async function exportPdfFromHtml(html, outPath) {
  const win = new BrowserWindow({
    show: false,
    webPreferences: {
      sandbox: true
    }
  });

  await win.loadURL(
    "data:text/html;charset=utf-8," + encodeURIComponent(html)
  );

  // Give fonts/layout a moment. (We’ll tighten later with explicit font loading hooks.)
  await new Promise((r) => setTimeout(r, 200));

  const pdfBuf = await win.webContents.printToPDF({
    printBackground: true,
    pageSize: "A4",
    margins: { marginType: "default" }
  });

  fs.writeFileSync(outPath, pdfBuf);
  win.destroy();
}

function createMainWindow() {
  mainWindow = new BrowserWindow({
    width: 1200,
    height: 800,
    webPreferences: {
      contextIsolation: true,
      nodeIntegration: false,
      sandbox: true,
      preload: path.join(__dirname, "preload.js")
    }
  });

  const devUrl = process.env.VITE_DEV_SERVER_URL;
  if (devUrl) mainWindow.loadURL(devUrl);
  else mainWindow.loadFile(path.join(__dirname, "..", "dist", "renderer", "index.html"));
}

app.whenReady().then(() => {
  createMainWindow();

  app.on("activate", () => {
    if (BrowserWindow.getAllWindows().length === 0) createMainWindow();
  });
});

app.on("window-all-closed", () => {
  if (process.platform !== "darwin") app.quit();
});

ipcMain.handle("markbookd.request", async (_evt, req) => {
  return sidecarRequest(req.method, req.params);
});

ipcMain.handle("markbookd.restart", async () => {
  stopSidecar();
  return { ok: true };
});

ipcMain.handle("markbookd.meta", async () => {
  return {
    running: Boolean(sidecar),
    pid: sidecar?.pid ?? null,
    path: sidecarPathUsed
  };
});

ipcMain.handle("prefs.get", async () => {
  return readPrefs();
});

ipcMain.handle("prefs.addRecentWorkspace", async (_evt, payload) => {
  const wsPath = payload?.path;
  const next = writePrefs(addRecentWorkspace(readPrefs(), wsPath));
  return { ok: true, prefs: next };
});

ipcMain.handle("prefs.setLastWorkspace", async (_evt, payload) => {
  const wsPath = payload?.path;
  let cur = readPrefs();
  cur = addRecentWorkspace(cur, wsPath);
  cur = { ...cur, lastWorkspace: wsPath || null };
  const next = writePrefs(cur);
  return { ok: true, prefs: next };
});

ipcMain.handle("workspace.select", async () => {
  const res = await dialog.showOpenDialog({
    properties: ["openDirectory", "createDirectory"]
  });
  if (res.canceled || !res.filePaths[0]) return null;
  const selected = res.filePaths[0];
  // Tell sidecar to use it (creates db as needed).
  await sidecarRequest("workspace.select", { path: selected });
  // Persist in prefs for quick reopen.
  writePrefs({ ...addRecentWorkspace(readPrefs(), selected), lastWorkspace: selected });
  return selected;
});

ipcMain.handle("legacy.selectClassFolder", async () => {
  const res = await dialog.showOpenDialog({
    properties: ["openDirectory"]
  });
  if (res.canceled || !res.filePaths[0]) return null;
  return res.filePaths[0];
});

ipcMain.handle("files.pickSave", async (_evt, payload) => {
  if (process.env.MARKBOOK_E2E_PICK_SAVE_PATH) {
    return process.env.MARKBOOK_E2E_PICK_SAVE_PATH;
  }

  const options = {
    title: payload?.title || "Save File",
    defaultPath: payload?.defaultPath,
    filters: Array.isArray(payload?.filters) ? payload.filters : undefined,
  };
  const res = mainWindow
    ? await dialog.showSaveDialog(mainWindow, options)
    : await dialog.showSaveDialog(options);
  if (res.canceled || !res.filePath) return null;
  return res.filePath;
});

ipcMain.handle("files.pickOpen", async (_evt, payload) => {
  if (process.env.MARKBOOK_E2E_PICK_OPEN_PATH) {
    return process.env.MARKBOOK_E2E_PICK_OPEN_PATH;
  }

  const options = {
    title: payload?.title || "Open File",
    properties: ["openFile"],
    filters: Array.isArray(payload?.filters) ? payload.filters : undefined,
  };
  const res = mainWindow
    ? await dialog.showOpenDialog(mainWindow, options)
    : await dialog.showOpenDialog(options);
  if (res.canceled || !res.filePaths[0]) return null;
  return res.filePaths[0];
});

ipcMain.handle("pdf.exportHtml", async (_evt, payload) => {
  const { html, outPath } = payload || {};
  if (!html || !outPath) throw new Error("missing html or outPath");
  await exportPdfFromHtml(html, outPath);
  return { ok: true };
});

ipcMain.handle("pdf.exportHtmlWithSaveDialog", async (_evt, payload) => {
  const { html, defaultFilename } = payload || {};
  if (!html) throw new Error("missing html");

  const options = {
    title: "Export PDF",
    defaultPath: defaultFilename || "markbook-report.pdf",
    filters: [{ name: "PDF", extensions: ["pdf"] }]
  };
  const res = mainWindow
    ? await dialog.showSaveDialog(mainWindow, options)
    : await dialog.showSaveDialog(options);
  if (res.canceled || !res.filePath) return { canceled: true };

  await exportPdfFromHtml(html, res.filePath);
  return { canceled: false, path: res.filePath };
});
