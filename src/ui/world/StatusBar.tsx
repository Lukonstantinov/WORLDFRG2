import { useUIStore } from "@state/uiStore";
import { useWorldStore } from "@state/worldStore";

export function StatusBar() {
  const statusText = useUIStore((s) => s.statusText);
  const meta = useWorldStore((s) => s.meta);

  return (
    <div style={{
      height: 24, display: "flex", alignItems: "center", gap: 16,
      padding: "0 12px", background: "#0a0f18", borderTop: "1px solid #1a2535",
      fontSize: 11, color: "#4a6080", flexShrink: 0,
    }}>
      <span style={{ color: "#6a8aa8" }}>{statusText}</span>
      {meta && (
        <span style={{ marginLeft: "auto", fontFamily: "monospace", fontSize: 10, color: "#3a5068" }}>
          {meta.grid_width}\u00D7{meta.grid_height} | {Math.ceil(meta.grid_width / 128)}\u00D7{Math.ceil(meta.grid_height / 128)} tiles
        </span>
      )}
    </div>
  );
}
