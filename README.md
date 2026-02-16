# MarkBook Classic (Desktop)

Electron + Bun + Rust sidecar skeleton for the MarkBook Classic offline desktop rewrite.

## Prereqs
- Bun (installed)
- Rust toolchain (install via rustup) for building the sidecar

## Dev
1. `bun install`
2. Build sidecar (once):
   - `bun run sidecar:build:debug`
3. Run desktop app:
   - `bun run dev`

If the sidecar binary isn't built yet, the UI still launches and will show a sidecar error until you build it.

## Project Layout
- `apps/desktop`: Electron app (main + preload + React renderer)
- `packages/schema`: shared Zod schemas + IPC types
- `packages/core`: shared pure TypeScript logic (calculations later)
- `packages/reports`: HTML/CSS report templates later
- `rust/markbookd`: Rust sidecar process (SQLite + import/export later)

