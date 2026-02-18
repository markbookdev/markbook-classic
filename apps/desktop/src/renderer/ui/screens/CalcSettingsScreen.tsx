import React, { useEffect, useMemo, useState } from "react";
import {
  CalcConfigClearOverrideResultSchema,
  CalcConfigGetResultSchema,
  CalcConfigUpdateResultSchema
} from "@markbook/schema";
import { requestParsed } from "../state/workspace";

function clampInt(n: number, lo: number, hi: number) {
  return Math.max(lo, Math.min(hi, Math.trunc(n)));
}

function parseIntOr(prev: number, s: string) {
  const t = s.trim();
  if (!t) return prev;
  const n = Number(t);
  if (!Number.isFinite(n)) return prev;
  return Math.trunc(n);
}

export function CalcSettingsScreen(props: { onError: (msg: string | null) => void }) {
  const [loading, setLoading] = useState(false);
  const [source, setSource] = useState<{ basePresent: boolean; overridePresent: boolean } | null>(
    null
  );
  const [roff, setRoff] = useState(true);
  const [modeActiveLevels, setModeActiveLevels] = useState(4);
  const [modeVals, setModeVals] = useState<number[]>(Array.from({ length: 22 }, () => 0));
  const [modeSymbols, setModeSymbols] = useState<string[]>(Array.from({ length: 22 }, () => ""));

  const enabledCount = useMemo(() => clampInt(modeActiveLevels, 0, 21) + 1, [modeActiveLevels]);

  async function load() {
    setLoading(true);
    props.onError(null);
    try {
      const res = await requestParsed("calc.config.get", {}, CalcConfigGetResultSchema);
      setSource(res.source);
      setRoff(res.roff);
      setModeActiveLevels(clampInt(res.modeActiveLevels, 0, 21));
      setModeVals(res.modeVals.slice(0, 22));
      setModeSymbols(res.modeSymbols.slice(0, 22));
    } catch (e: any) {
      props.onError(e?.message ?? String(e));
      setSource(null);
    } finally {
      setLoading(false);
    }
  }

  useEffect(() => {
    void load();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  async function saveOverrides() {
    props.onError(null);
    try {
      await requestParsed(
        "calc.config.update",
        {
          roff,
          modeActiveLevels,
          modeVals,
          modeSymbols
        },
        CalcConfigUpdateResultSchema
      );
      await load();
    } catch (e: any) {
      props.onError(e?.message ?? String(e));
    }
  }

  async function clearOverrides() {
    const ok = confirm("Clear calc overrides and revert to base settings?");
    if (!ok) return;
    props.onError(null);
    try {
      await requestParsed("calc.config.clearOverride", {}, CalcConfigClearOverrideResultSchema);
      await load();
    } catch (e: any) {
      props.onError(e?.message ?? String(e));
    }
  }

  return (
    <div data-testid="calc-settings-screen" style={{ padding: 24, maxWidth: 980 }}>
      <div style={{ fontWeight: 800, fontSize: 22, marginBottom: 10 }}>Calculation Settings</div>

      <div style={{ marginBottom: 12, fontSize: 12, color: "#555" }}>
        {source ? (
          <>
            Base settings: {source.basePresent ? "present" : "missing"} | Overrides:{" "}
            {source.overridePresent ? "present" : "none"}
          </>
        ) : (
          <>Loading settings…</>
        )}
      </div>

      <div
        style={{
          border: "1px solid #ddd",
          borderRadius: 10,
          padding: 16,
          marginBottom: 16
        }}
      >
        <div style={{ display: "flex", alignItems: "center", gap: 16, flexWrap: "wrap" }}>
          <label style={{ display: "flex", alignItems: "center", gap: 8 }}>
            <input
              data-testid="calc-settings-roff"
              type="checkbox"
              checked={roff}
              onChange={(e) => setRoff(e.currentTarget.checked)}
            />
            RoundOff (roff)
          </label>

          <label style={{ display: "flex", alignItems: "center", gap: 8 }}>
            Active levels
            <input
              data-testid="calc-settings-levels"
              value={String(modeActiveLevels)}
              onChange={(e) => setModeActiveLevels(parseIntOr(modeActiveLevels, e.currentTarget.value))}
              style={{ width: 60, padding: "4px 6px" }}
            />
          </label>

          <div style={{ marginLeft: "auto", display: "flex", gap: 8 }}>
            <button data-testid="calc-settings-save" onClick={() => void saveOverrides()} disabled={loading}>
              Save Overrides
            </button>
            <button data-testid="calc-settings-clear" onClick={() => void clearOverrides()} disabled={loading}>
              Clear Overrides
            </button>
            <button onClick={() => void load()} disabled={loading}>
              Reload
            </button>
          </div>
        </div>

        <div style={{ marginTop: 12, overflow: "auto" }}>
          <table style={{ width: "100%", borderCollapse: "collapse" }}>
            <thead>
              <tr style={{ background: "#fafafa", borderBottom: "1px solid #eee" }}>
                <th style={{ textAlign: "left", padding: 8, width: 60 }}>Level</th>
                <th style={{ textAlign: "left", padding: 8, width: 120 }}>Threshold</th>
                <th style={{ textAlign: "left", padding: 8 }}>Symbol</th>
              </tr>
            </thead>
            <tbody>
              {Array.from({ length: 22 }, (_, i) => i).map((i) => {
                const enabled = i < enabledCount;
                return (
                  <tr key={i} style={{ borderBottom: "1px solid #f0f0f0", opacity: enabled ? 1 : 0.5 }}>
                    <td style={{ padding: 8 }}>{i}</td>
                    <td style={{ padding: 8 }}>
                      <input
                        value={String(modeVals[i] ?? 0)}
                        disabled={!enabled}
                        onChange={(e) => {
                          const next = modeVals.slice();
                          next[i] = parseIntOr(next[i] ?? 0, e.currentTarget.value);
                          setModeVals(next);
                        }}
                        style={{ width: 100, padding: "4px 6px" }}
                      />
                    </td>
                    <td style={{ padding: 8 }}>
                      <input
                        value={modeSymbols[i] ?? ""}
                        disabled={!enabled}
                        onChange={(e) => {
                          const next = modeSymbols.slice();
                          next[i] = e.currentTarget.value;
                          setModeSymbols(next);
                        }}
                        style={{ width: "100%", padding: "4px 6px" }}
                      />
                    </td>
                  </tr>
                );
              })}
            </tbody>
          </table>
        </div>
      </div>

      <div style={{ fontSize: 12, color: "#666" }}>
        These settings affect Mode and blended calculations. Base settings are loaded from legacy
        <code> *_USR.CFG</code> when present; overrides take precedence.
      </div>
    </div>
  );
}

