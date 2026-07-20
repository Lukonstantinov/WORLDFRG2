import { useUIStore } from "@state/uiStore";

const LAND_BANDS = [
  { label: "8848m", color: "#ffffff" },
  { label: "7000m", color: "#b4aaa0" },
  { label: "5000m", color: "#8c6432" },
  { label: "3000m", color: "#aaa03c" },
  { label: "1500m", color: "#56943c" },
  { label: "0m",    color: "#387632" },
];

const SEA_BANDS = [
  { label: "Shelf",    color: "#1d78c4" },
  { label: "Slope",    color: "#144a8c" },
  { label: "Abyssal",  color: "#050f2e" },
];

export function ElevationLegend() {
  const activeLayer = useUIStore((s) => s.activeLayer);

  if (activeLayer !== "elevation" && activeLayer !== "terrain") return null;

  return (
    <div style={container}>
      <div style={title}>Elevation</div>

      {/* Land gradient */}
      <div style={{ display: "flex", gap: 4 }}>
        <div style={gradientBar}>
          {LAND_BANDS.map((b, i) => (
            <div key={i} style={{ flex: 1, background: b.color }} />
          ))}
        </div>
        <div style={labelsCol}>
          {LAND_BANDS.map((b, i) => (
            <div key={i} style={labelText}>{b.label}</div>
          ))}
        </div>
      </div>

      {/* Sea depth */}
      <div style={{ ...title, marginTop: 8 }}>Sea Depth</div>
      {SEA_BANDS.map((b, i) => (
        <div key={i} style={swatchRow}>
          <div style={{ ...swatch, background: b.color }} />
          <span style={labelText}>{b.label}</span>
        </div>
      ))}
    </div>
  );
}

const container: React.CSSProperties = {
  position: "absolute", bottom: 8, left: 8,
  background: "rgba(10, 15, 24, 0.94)",
  border: "1px solid #1e2e42", borderRadius: 6,
  padding: 10, minWidth: 100, zIndex: 20,
  backdropFilter: "blur(8px)",
};

const title: React.CSSProperties = {
  fontSize: 10, color: "#8890a0", textTransform: "uppercase",
  letterSpacing: 1, marginBottom: 4,
};

const gradientBar: React.CSSProperties = {
  width: 16, height: 100, display: "flex", flexDirection: "column",
  borderRadius: 2, overflow: "hidden", border: "1px solid #2a3a50",
};

const labelsCol: React.CSSProperties = {
  display: "flex", flexDirection: "column", justifyContent: "space-between",
};

const labelText: React.CSSProperties = {
  fontSize: 9, color: "#8890a0", fontFamily: "monospace",
};

const swatchRow: React.CSSProperties = {
  display: "flex", alignItems: "center", gap: 6, marginBottom: 2,
};

const swatch: React.CSSProperties = {
  width: 14, height: 10, borderRadius: 2, border: "1px solid #2a3a50",
};
