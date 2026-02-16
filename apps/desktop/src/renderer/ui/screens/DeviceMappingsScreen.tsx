import React, { useEffect, useMemo, useState } from "react";
import { DevicesListResultSchema, DevicesUpdateResultSchema } from "@markbook/schema";
import { requestParsed } from "../state/workspace";

type DeviceRow = {
  studentId: string;
  displayName: string;
  sortOrder: number;
  active: boolean;
  deviceCode: string;
  rawLine: string;
};

export function DeviceMappingsScreen(props: {
  selectedClassId: string;
  onError: (msg: string | null) => void;
}) {
  const [rows, setRows] = useState<DeviceRow[]>([]);
  const [busyRowId, setBusyRowId] = useState<string | null>(null);
  const [dirtyCodes, setDirtyCodes] = useState<Record<string, string>>({});
  const [status, setStatus] = useState("");

  const sortedRows = useMemo(
    () => [...rows].sort((a, b) => a.sortOrder - b.sortOrder),
    [rows]
  );

  async function load() {
    props.onError(null);
    setStatus("");
    try {
      const res = await requestParsed(
        "devices.list",
        { classId: props.selectedClassId },
        DevicesListResultSchema
      );
      setRows(res.devices as DeviceRow[]);
      setDirtyCodes({});
    } catch (e: any) {
      props.onError(e?.message ?? String(e));
    }
  }

  useEffect(() => {
    void load();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [props.selectedClassId]);

  function codeFor(row: DeviceRow) {
    return dirtyCodes[row.studentId] ?? row.deviceCode ?? "";
  }

  async function saveRow(row: DeviceRow) {
    const nextCode = codeFor(row).trim();
    setBusyRowId(row.studentId);
    setStatus("");
    props.onError(null);
    try {
      await requestParsed(
        "devices.update",
        {
          classId: props.selectedClassId,
          studentId: row.studentId,
          deviceCode: nextCode,
          rawLine: ""
        },
        DevicesUpdateResultSchema
      );
      setStatus(`Saved device mapping for ${row.displayName}`);
      await load();
    } catch (e: any) {
      props.onError(e?.message ?? String(e));
    } finally {
      setBusyRowId(null);
    }
  }

  return (
    <div data-testid="devices-screen" style={{ padding: 24 }}>
      <div style={{ fontWeight: 800, fontSize: 22, marginBottom: 12 }}>Device Mappings</div>
      <div style={{ display: "flex", gap: 8, marginBottom: 12 }}>
        <button data-testid="devices-reload-btn" onClick={() => void load()} disabled={Boolean(busyRowId)}>
          Reload
        </button>
      </div>

      <div style={{ border: "1px solid #ddd", borderRadius: 10, padding: 12 }}>
        {sortedRows.length === 0 ? (
          <div style={{ color: "#666" }}>(no students)</div>
        ) : (
          <table style={{ width: "100%", borderCollapse: "collapse", fontSize: 13 }}>
            <thead>
              <tr>
                <th style={{ textAlign: "left" }}>Student</th>
                <th style={{ textAlign: "left" }}>Active</th>
                <th style={{ textAlign: "left" }}>Device Code</th>
                <th style={{ textAlign: "left" }}>Save</th>
              </tr>
            </thead>
            <tbody>
              {sortedRows.map((row) => {
                const busy = busyRowId === row.studentId;
                return (
                  <tr key={row.studentId} data-testid={`devices-row-${row.studentId}`}>
                    <td>{row.displayName}</td>
                    <td>{row.active ? "Yes" : "No"}</td>
                    <td>
                      <input
                        data-testid={`devices-code-input-${row.studentId}`}
                        value={codeFor(row)}
                        onChange={(e) =>
                          setDirtyCodes((cur) => ({
                            ...cur,
                            [row.studentId]: e.currentTarget.value
                          }))
                        }
                        disabled={busy}
                        style={{ width: "100%" }}
                      />
                    </td>
                    <td>
                      <button
                        data-testid={`devices-save-btn-${row.studentId}`}
                        onClick={() => void saveRow(row)}
                        disabled={busy}
                      >
                        {busy ? "Saving..." : "Save"}
                      </button>
                    </td>
                  </tr>
                );
              })}
            </tbody>
          </table>
        )}
      </div>

      <div style={{ marginTop: 8, color: "#666", fontSize: 12 }}>
        Clearing a device code and saving removes that mapping.
      </div>
      {status ? <div style={{ marginTop: 8, color: "#1a5" }}>{status}</div> : null}
    </div>
  );
}
