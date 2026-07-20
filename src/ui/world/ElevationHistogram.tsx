import { useState, useEffect } from "react";
import { getElevationDistribution, type ElevationBand } from "@bridge";
import { useWorldStore } from "@state/worldStore";
import { useViewportStore } from "@state/viewportStore";

const BAND_COLORS = [
  "#387632", "#4a8a3c", "#6a9e3c", "#aaa03c", "#c4a040",
  "#8c6432", "#7a5030", "#b4aaa0", "#e8e8f0",
];

export function ElevationHistogram() {
  const [bands, setBands] = useState<ElevationBand[]>([]);
  const [collapsed, setCollapsed] = useState(true);
  const meta = useWorldStore((s) => s.meta);
  const tileVersion = useViewportStore((s) => s.tileVersion);

  const refresh = () => {
    if (!meta) return;
    getElevationDistribution()
      .then(setBands)
      .catch(console.error);
  };

  useEffect(() => {
    if (!collapsed) refresh();
  }, [collapsed, meta, tileVersion]);

  if (!meta) return null;

  return (
    <div style={container}>
      <div
        style={header}
        onClick={() => setCollapsed(!collapsed)}
      >
        <span>{collapsed ? "+" : "-"}</span>
        <span style={{ marginLeft: 6 }}>Elevation Distribution</span>
      </div>

      {!collapsed && (
        <div style={{ padding: "6px 0" }}>
          {bands.map((band, i) => (
            <div key={i} style={barRow}>
              <div style={barLabel}>{band.label}</div>
              <div style={barTrack}>
                <div style={{
                  ...barFill,
                  width: `${Math.max(1, band.percentage)}%`,
                  background: BAND_COLORS[i] || "#888",
                }} />
              </div>
              <div style={barPct}>{band.percentage.toFixed(1)}%</div>
            </div>
          ))}
          <button onClick={refresh} style={refreshBtn}>Refresh</button>
        </div>
      )}
    </div>
  );
}

const container: React.CSSProperties = {
  position: "absolute", bottom: 8, left: 140,
  background: "rgba(10, 15, 24, 0.94)",
  border: "1px solid #1e2e42", borderRadius: 6,
  padding: 8, minWidth: 200, zIndex: 20,
  backdropFilter: "blur(8px)",
};

const header: React.CSSProperties = {
  fontSize: 11, color: "#a0b0c0", cursor: "pointer",
  display: "flex", alignItems: "center",
};

const barRow: React.CSSProperties = {
  display: "flex", alignItems: "center", gap: 4, marginBottom: 3,
};

const barLabel: React.CSSProperties = {
  fontSize: 9, color: "#8890a0", width: 60, textAlign: "right",
  fontFamily: "monospace",
};

const barTrack: React.CSSProperties = {
  flex: 1, height: 10, background: "#1a2234", borderRadius: 2,
  overflow: "hidden",
};

const barFill: React.CSSProperties = {
  height: "100%", borderRadius: 2, transition: "width 0.3s",
};

const barPct: React.CSSProperties = {
  fontSize: 9, color: "#8890a0", width: 40, textAlign: "right",
  fontFamily: "monospace",
};

const refreshBtn: React.CSSProperties = {
  marginTop: 4, padding: "3px 8px", fontSize: 10,
  background: "#1a2a40", border: "1px solid #2a4060",
  borderRadius: 3, color: "#8090b0", cursor: "pointer",
};
