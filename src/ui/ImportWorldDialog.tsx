import { useState } from "react";
import { importWorldLayers, getOverlays } from "@bridge";
import { useWorldStore } from "@state/worldStore";
import { useViewportStore } from "@state/viewportStore";
import { useUIStore } from "@state/uiStore";

const GROUPS: { id: string; label: string; desc: string }[] = [
  { id: "terrain", label: "Terrain & Elevation", desc: "land/sea, elevation, sea depth, shelves, plates, volcanoes" },
  { id: "climate", label: "Climate", desc: "winds, currents, temperature, precipitation, Köppen, salinity" },
  { id: "hydrology", label: "Rivers & Lakes", desc: "river and lake overlays" },
  { id: "soil", label: "Soil & Fertility", desc: "soil types, fertility, fisheries" },
  { id: "hazards", label: "Hazards", desc: "sharks, shipworm, storms, reefs, disease" },
  { id: "goods", label: "Trade Goods", desc: "trade-good belts + the goods catalogue" },
];

/** Pick a .worldforge file and copy the chosen layer groups into the current
 *  world (same grid size required); everything unticked is left as-is. */
export function ImportWorldDialog({ onClose }: { onClose: () => void }) {
  const [selected, setSelected] = useState<Set<string>>(new Set(["terrain", "climate"]));
  const [busy, setBusy] = useState(false);
  const setStatus = useUIStore((s) => s.setStatus);
  const invalidateTiles = useViewportStore((s) => s.invalidateTiles);
  const { setRivers, setLakes } = useWorldStore();

  const toggle = (id: string) =>
    setSelected((prev) => {
      const next = new Set(prev);
      next.has(id) ? next.delete(id) : next.add(id);
      return next;
    });

  const handleImport = async () => {
    setBusy(true);
    try {
      let path: string | null = null;
      try {
        const { open } = await import("@tauri-apps/plugin-dialog");
        const result = await open({
          filters: [{ name: "WorldForge", extensions: ["worldforge", "db"] }],
          title: "Choose source world",
        });
        if (result) path = result as string;
      } catch {
        const input = prompt("Enter path to source .worldforge file:");
        if (input) path = input;
      }
      if (!path) { setBusy(false); return; }

      setStatus("Importing layers...");
      await importWorldLayers(path, Array.from(selected));
      invalidateTiles();
      if (selected.has("hydrology")) {
        const ov = await getOverlays();
        setRivers(ov.rivers ?? []);
        setLakes(ov.lakes ?? []);
      }
      setStatus(`Imported: ${Array.from(selected).join(", ")}`);
      onClose();
    } catch (err) {
      console.error("Import failed:", err);
      alert("Import failed: " + err);
    }
    setBusy(false);
  };

  return (
    <div style={{
      position: "absolute", inset: 0, display: "flex",
      alignItems: "center", justifyContent: "center",
      background: "rgba(0,0,0,0.75)", zIndex: 100,
    }}>
      <div style={{
        background: "#111820", border: "1px solid #1e2e42", borderRadius: 10,
        padding: "24px 28px", minWidth: 400, maxHeight: "80%", overflowY: "auto",
        boxShadow: "0 12px 40px rgba(0,0,0,0.5)",
      }}>
        <h2 style={{ margin: "0 0 8px", color: "#c0d8f0", fontSize: 17, fontWeight: 600 }}>
          Import World Layers
        </h2>
        <div style={{ color: "#5a7898", fontSize: 11, marginBottom: 14 }}>
          Copy chosen layers from another world of the SAME grid size into this one.
          Unticked layers keep their current data.
        </div>

        <div style={{ display: "flex", flexDirection: "column", gap: 6, marginBottom: 16 }}>
          {GROUPS.map((g) => (
            <label key={g.id} style={{ display: "flex", alignItems: "flex-start", gap: 8, cursor: "pointer" }}>
              <input type="checkbox" checked={selected.has(g.id)} onChange={() => toggle(g.id)}
                style={{ accentColor: "#4a90d0", marginTop: 2 }} />
              <span>
                <span style={{ color: "#a0b8d0", fontSize: 12, fontWeight: 600 }}>{g.label}</span>
                <span style={{ color: "#506080", fontSize: 11, display: "block" }}>{g.desc}</span>
              </span>
            </label>
          ))}
        </div>

        <div style={{ display: "flex", gap: 8, justifyContent: "flex-end" }}>
          <button onClick={onClose} disabled={busy}
            style={{ padding: "8px 16px", borderRadius: 6, border: "1px solid #1e2e42", background: "#0d1219", color: "#7090b0", cursor: "pointer", fontSize: 13 }}>
            Cancel
          </button>
          <button onClick={handleImport} disabled={busy || selected.size === 0}
            style={{ padding: "8px 18px", borderRadius: 6, border: "none", background: busy ? "#1a3050" : "#2060a0", color: "#fff", cursor: busy ? "wait" : "pointer", fontSize: 13, fontWeight: 600 }}>
            {busy ? "Importing..." : "Choose World & Import"}
          </button>
        </div>
      </div>
    </div>
  );
}
