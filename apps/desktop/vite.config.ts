import path from "node:path";
import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

export default defineConfig({
  root: path.join(__dirname, "src/renderer"),
  // The renderer is loaded via `file://` in production (Electron `loadFile`).
  // Use a relative base so built asset URLs resolve correctly.
  base: "./",
  plugins: [react()],
  build: {
    outDir: path.join(__dirname, "dist/renderer"),
    emptyOutDir: true
  },
  server: {
    port: 5173,
    strictPort: true
  }
});
