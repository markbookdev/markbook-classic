import React from "react";
import { AppShell } from "./app/AppShell";

// Attach window.__markbookTest helpers for Playwright (no-op for normal users).
import "./state/e2e";

export function App() {
  return <AppShell />;
}
