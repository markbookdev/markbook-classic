import React from "react";

export function ComingSoon(props: { title: string; body?: string }) {
  return (
    <div style={{ padding: 24, maxWidth: 760 }}>
      <div style={{ fontWeight: 700, fontSize: 18, marginBottom: 8 }}>{props.title}</div>
      <div style={{ color: "#444", marginBottom: 12 }}>
        {props.body ?? "This screen is coming soon."}
      </div>
      <div
        style={{
          border: "1px solid #ddd",
          borderRadius: 8,
          padding: 16,
          background: "#fafafa",
          color: "#555",
          fontSize: 13,
          lineHeight: 1.35
        }}
      >
        <div style={{ fontWeight: 700, marginBottom: 6 }}>Planned</div>
        <ul style={{ margin: 0, paddingLeft: 18 }}>
          <li>Legacy import parity (companions + calculations)</li>
          <li>Keyboard-first UX and bulk editing</li>
          <li>Professional PDF reports</li>
        </ul>
      </div>
    </div>
  );
}

