import React, { useMemo } from "react";
import {
  actionsForGroup,
  LEGACY_MENU_GROUP_LABELS,
  LEGACY_MENU_GROUP_ORDER,
  type LegacyMenuAction
} from "../state/actionRegistry";

function routeLabel(action: LegacyMenuAction): string {
  if (!action.screenRoute) return "(n/a)";
  return action.screenRoute;
}

export function LegacyActionsMapScreen() {
  const rows = useMemo(() => {
    return LEGACY_MENU_GROUP_ORDER.flatMap((group) =>
      actionsForGroup(group).map((action) => ({
        ...action,
        groupLabel: LEGACY_MENU_GROUP_LABELS[group]
      }))
    );
  }, []);

  return (
    <div data-testid="legacy-actions-map-screen" style={{ padding: 24 }}>
      <div data-testid="legacy-actions-map-title" style={{ fontSize: 22, fontWeight: 800, marginBottom: 10 }}>
        Legacy Actions Map
      </div>
      <div style={{ color: "#555", marginBottom: 12, maxWidth: 920 }}>
        Canonical mapping of legacy menu actions to current desktop routes. Pending rows are
        explicitly disabled in menus with the tooltip text "Not implemented yet".
      </div>
      <div style={{ overflow: "auto", border: "1px solid #ddd", borderRadius: 8 }}>
        <table style={{ width: "100%", borderCollapse: "collapse" }}>
          <thead>
            <tr>
              <th style={{ textAlign: "left", padding: 8 }}>Group</th>
              <th style={{ textAlign: "left", padding: 8 }}>Action</th>
              <th style={{ textAlign: "left", padding: 8 }}>Route</th>
              <th style={{ textAlign: "left", padding: 8 }}>Status</th>
              <th style={{ textAlign: "left", padding: 8 }}>Reason</th>
            </tr>
          </thead>
          <tbody>
            {rows.map((row) => (
              <tr
                key={row.id}
                data-testid={`legacy-action-map-row-${row.id}`}
                style={{ borderTop: "1px solid #eee" }}
              >
                <td style={{ padding: 8 }}>{row.groupLabel}</td>
                <td style={{ padding: 8 }}>{row.label}</td>
                <td style={{ padding: 8, fontFamily: "monospace" }}>{routeLabel(row)}</td>
                <td style={{ padding: 8 }}>{row.implemented ? "implemented" : "pending"}</td>
                <td style={{ padding: 8 }}>
                  {row.implemented ? row.enabledReason : row.pendingReason}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  );
}
