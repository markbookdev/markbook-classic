const { contextBridge, ipcRenderer } = require("electron");

contextBridge.exposeInMainWorld("markbook", {
  selectWorkspace: () => ipcRenderer.invoke("workspace.select"),
  selectLegacyClassFolder: () => ipcRenderer.invoke("legacy.selectClassFolder"),
  request: (method, params) =>
    ipcRenderer.invoke("markbookd.request", { method, params }),
  restartSidecar: () => ipcRenderer.invoke("markbookd.restart"),
  getSidecarMeta: () => ipcRenderer.invoke("markbookd.meta"),
  prefs: {
    get: () => ipcRenderer.invoke("prefs.get"),
    addRecentWorkspace: (path) =>
      ipcRenderer.invoke("prefs.addRecentWorkspace", { path }),
    setLastWorkspace: (path) =>
      ipcRenderer.invoke("prefs.setLastWorkspace", { path })
  },
  exportPdfHtml: (html, outPath) =>
    ipcRenderer.invoke("pdf.exportHtml", { html, outPath }),
  exportPdfHtmlWithSaveDialog: (html, defaultFilename) =>
    ipcRenderer.invoke("pdf.exportHtmlWithSaveDialog", { html, defaultFilename })
});
